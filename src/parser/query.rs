use super::*;

/// 查找指定语法项的参数值
///
/// 兼容`DataWindow::Describe`参数
pub fn find<'a, 'b: 'a, 'c>(syn: &'a DWSyntax<'b>, input: &'c str) -> Result<'c, Option<&'a Value<'b>>> {
    let SelectResult {
        root,
        key
    } = select(syn, input)?;
    if key.is_empty() {
        return Ok(None);
    }
    let values = match root {
        SelectRoot::DataWindow => &syn.datawindow,
        SelectRoot::Header => &syn.header,
        SelectRoot::Summary => &syn.summary,
        SelectRoot::Footer => &syn.footer,
        SelectRoot::Detail => &syn.detail,
        SelectRoot::Item(index) => &syn.items[index].values,
        SelectRoot::ItemTable => &syn.table.values,
        SelectRoot::ItemTableColumn(index) => &syn.table.columns[index].values
    };
    Ok(values.get(&key))
}

/// 修改语法项的参数值
///
/// 兼容`DataWindow::Modify`参数
pub fn modify<'a, 'b: 'a, 'c>(syn: &'a mut DWSyntax<'b>, input: &'c str) -> Result<'c, ()> {
    enum ModifyKind<'a> {
        Assign(&'a str, Value<'a>),
        Create(SumItem<'a>),
        Destroy(&'a str)
    }
    fn key(input: &str) -> ParseResult<&str> {
        take_while1(|c: char| c.is_alphanumeric() || c == '#' || c == '.' || c == '_')(input)
    }
    fn assign(input: &str) -> ParseResult<ModifyKind> {
        fn value(input: &str) -> ParseResult<Value> {
            cut(alt((value::string, value::literal, value::number, value::map, value::list, fail)))(input)
        }
        separated_pair(key, delimited(multispace0, tag("="), multispace0), value)
            .map(|(key, val)| ModifyKind::Assign(key, val))
            .parse(input)
    }
    fn create(input: &str) -> ParseResult<ModifyKind> {
        #[cfg(feature = "case_insensitive")]
        let (input, _) = tag_no_case("create")(input)?;
        #[cfg(not(feature = "case_insensitive"))]
        let (input, _) = tag("create")(input)?;
        let (input, _) = multispace1(input)?;
        item.map(ModifyKind::Create).parse(input)
    }
    fn destroy(input: &str) -> ParseResult<ModifyKind> {
        #[cfg(feature = "case_insensitive")]
        let (input, _) = tag_no_case("destroy")(input)?;
        #[cfg(not(feature = "case_insensitive"))]
        let (input, _) = tag("create")(input)?;
        let (input, _) = multispace1(input)?;
        key.map(ModifyKind::Destroy).parse(input)
    }
    let (_, modifies) = terminated(
        preceded(
            multispace0,
            separated_list1(
                alt((delimited(multispace0, tag(";"), multispace0), multispace1)),
                alt((assign, create, destroy, fail))
            )
        ),
        terminated(delimited(multispace0, opt(tag(";")), multispace0), eof)
    )(input)?;
    for kind in modifies {
        match kind {
            ModifyKind::Assign(selector, value) => {
                let SelectResult {
                    root,
                    key
                } = select(syn, selector)?;
                if key.is_empty() {
                    return Err(NomErr::Error(make_error(selector, ErrorKind::Fail)));
                }
                let values = match root {
                    SelectRoot::DataWindow => &mut syn.datawindow,
                    SelectRoot::Header => &mut syn.header,
                    SelectRoot::Summary => &mut syn.summary,
                    SelectRoot::Footer => &mut syn.footer,
                    SelectRoot::Detail => &mut syn.detail,
                    SelectRoot::Item(index) => {
                        if key == "name" {
                            syn.items[index].name =
                                value.as_literal().map(|v| Cow::clone(v).into_owned().into_key());
                        } else if key == "id" {
                            syn.items[index].id = value.as_number().map(|v| v as u32);
                        }
                        &mut syn.items[index].values
                    },
                    SelectRoot::ItemTable => &mut syn.table.values,
                    SelectRoot::ItemTableColumn(index) => {
                        if key == "name" {
                            syn.table.columns[index].name =
                                value.as_literal().map(|v| Cow::clone(v).into_owned().into_key());
                        }
                        &mut syn.table.columns[index].values
                    }
                };
                values.insert(key, value.to_owned());
            },
            ModifyKind::Create(new_item) => {
                let new_item = new_item.to_owned();
                match new_item {
                    SumItem::Item(new_item) => {
                        if new_item.kind == "datawindow" {
                            syn.datawindow = new_item.values;
                        } else if new_item.kind == "header" {
                            syn.header = new_item.values;
                        } else if new_item.kind == "summary" {
                            syn.summary = new_item.values;
                        } else if new_item.kind == "footer" {
                            syn.footer = new_item.values;
                        } else if new_item.kind == "detail" {
                            syn.detail = new_item.values;
                        } else {
                            for item in &mut syn.items {
                                if item.kind == new_item.kind &&
                                    (item.name == new_item.name || item.id == new_item.id) &&
                                    item.level == new_item.level
                                {
                                    *item = new_item;
                                    return Ok(());
                                }
                            }
                            syn.items.push(new_item);
                        }
                    },
                    SumItem::ItemData(new_item) => syn.data = new_item,
                    SumItem::ItemTable(new_item) => syn.table = new_item
                }
            },
            ModifyKind::Destroy(name) => {
                let SelectResult {
                    root,
                    key
                } = select(syn, name)?;
                if !key.is_empty() {
                    return Err(NomErr::Error(make_error(name, ErrorKind::Fail)));
                }
                match root {
                    SelectRoot::DataWindow => syn.datawindow.clear(),
                    SelectRoot::Header => syn.header.clear(),
                    SelectRoot::Summary => syn.summary.clear(),
                    SelectRoot::Footer => syn.footer.clear(),
                    SelectRoot::Detail => syn.detail.clear(),
                    SelectRoot::Item(index) => {
                        syn.items.remove(index);
                    },
                    SelectRoot::ItemTable => {
                        syn.table.columns.clear();
                        syn.table.values.clear();
                    },
                    SelectRoot::ItemTableColumn(index) => {
                        syn.table.columns.remove(index);
                        //删除引用的字段控件并刷新ID
                        let mut i = syn.items.len();
                        while i > 0 {
                            i -= 1;
                            let item = &mut syn.items[i];
                            if item.kind == "column" {
                                if let Some(id) = item.id {
                                    //删除控件
                                    if id as usize == index + 1 {
                                        drop(item);
                                        syn.items.remove(i);
                                    }
                                    //刷新ID
                                    else if id as usize > index + 1 {
                                        item.id = Some(id - 1);
                                        item.values.insert("id".into_key(), Value::Number((id - 1) as f64));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// 选取结果
struct SelectResult<'a> {
    root: SelectRoot,
    key: Key<'a>
}

/// 选取的语法项根元素
#[derive(Debug)]
enum SelectRoot {
    DataWindow,
    Header,
    Summary,
    Footer,
    Detail,
    Item(usize),
    ItemTable,
    ItemTableColumn(usize)
}

/// 选择指定语法项
fn select<'a, 'b>(syn: &DWSyntax<'a>, input: &'b str) -> Result<'b, SelectResult<'a>> {
    let (_, selector) = terminated(
        preceded(multispace0, separated_list1(tag("."), alt((name, index)).map(IntoKey::into_key))),
        terminated(multispace0, eof)
    )(input)?;
    if selector.len() == 0 {
        return Err(NomErr::Error(make_error(input, ErrorKind::Eof)));
    }
    let mut selector = selector.into_iter();
    let mut root = None;
    let mut prefix = String::new();

    //选取参数列表
    let name = selector.next().unwrap();
    //datawindow名称下的语法项
    if name == "datawindow" {
        //TODO
        //- tree
        if selector.len() == 0 {
            return Err(NomErr::Error(make_error(input, ErrorKind::Eof)));
        }
        let name = selector.next().unwrap();
        if name == "header" {
            root = Some(SelectRoot::Header);
            //datawindow.header.<group #>
            if selector.len() >= 1 {
                let name = selector.next().unwrap();
                if let Ok(level) = name.parse() {
                    if let Some((index, _)) = find_group(&syn.items, level) {
                        root = Some(SelectRoot::Item(index));
                        prefix = "header".to_owned();
                    } else {
                        return Err(NomErr::Error(make_error(
                            //SAFETY
                            name.borrowed().unwrap(),
                            ErrorKind::Fail
                        )));
                    }
                } else {
                    prefix = name.as_ref().to_owned();
                }
            }
        } else if name == "footer" {
            root = Some(SelectRoot::Footer);
            //datawindow.footer.<group #>
            if selector.len() >= 1 {
                let name = selector.next().unwrap();
                if let Ok(level) = name.parse() {
                    if let Some((index, _)) = find_group(&syn.items, level) {
                        root = Some(SelectRoot::Item(index));
                        prefix = "footer".to_owned();
                    } else {
                        return Err(NomErr::Error(make_error(
                            //SAFETY
                            name.borrowed().unwrap(),
                            ErrorKind::Fail
                        )));
                    }
                } else {
                    prefix = name.as_ref().to_owned();
                }
            }
        } else if name == "trailer" {
            let mut found = false;
            //datawindow.trailer.<group #>
            if selector.len() >= 1 {
                if let Ok(level) = selector.next().unwrap().parse() {
                    if let Some((index, _)) = find_group(&syn.items, level) {
                        root = Some(SelectRoot::Item(index));
                        prefix = "trailer".to_owned();
                        found = true;
                    }
                }
            }
            if !found {
                return Err(NomErr::Error(make_error(
                    //SAFETY
                    name.borrowed().unwrap(),
                    ErrorKind::Fail
                )));
            }
        } else if name == "group" {
            let mut found = false;
            //datawindow.group.<group #>
            if selector.len() >= 1 {
                if let Ok(level) = selector.next().unwrap().parse() {
                    if let Some((index, _)) = find_group(&syn.items, level) {
                        root = Some(SelectRoot::Item(index));
                        found = true;
                    }
                }
            }
            if !found {
                return Err(NomErr::Error(make_error(
                    //SAFETY
                    name.borrowed().unwrap(),
                    ErrorKind::Fail
                )));
            }
        } else if name == "summary" {
            root = Some(SelectRoot::Summary);
        } else if name == "detail" {
            root = Some(SelectRoot::Detail);
        } else if name == "table" {
            root = Some(SelectRoot::ItemTable);
            //- datawindow.table.column.<column #>
            //- datawindow.table.column.<column name>
            if selector.len() >= 2 {
                let name = selector.next().unwrap();
                if name == "column" {
                    let name = selector.next().unwrap();
                    if let Ok(idx) = name.parse::<usize>() {
                        if idx > 0 && idx <= syn.table.columns.len() {
                            root = Some(SelectRoot::ItemTableColumn(idx - 1));
                        } else {
                            return Err(NomErr::Error(make_error(
                                //SAFETY
                                name.borrowed().unwrap(),
                                ErrorKind::Fail
                            )));
                        }
                    } else if let Some((index, _)) = find_table_column(&syn.table.columns, name.as_ref()) {
                        root = Some(SelectRoot::ItemTableColumn(index));
                    } else {
                        return Err(NomErr::Error(make_error(
                            //SAFETY
                            name.borrowed().unwrap(),
                            ErrorKind::Fail
                        )));
                    }
                } else {
                    prefix = name.as_ref().to_owned();
                }
            }
        } else {
            root = Some(SelectRoot::DataWindow);
            prefix = name.as_ref().to_owned();
        }
    }
    //具名的普通语法项
    else {
        match find_item(&syn.items, name.as_ref()) {
            Some((index, item)) => {
                root = Some(SelectRoot::Item(index));
                //特殊处理字段属性
                //- col.coltype
                //- col.dbname
                if selector.len() == 1 && item.kind == "column" && item.id.is_some() {
                    let name = selector.next().unwrap();
                    prefix = name.as_ref().to_owned();
                    if name == "coltype" || name == "dbname" {
                        let idx = item.id.unwrap() as usize;
                        if idx > 0 && idx <= syn.table.columns.len() {
                            root = Some(SelectRoot::ItemTableColumn(idx - 1));
                        } else {
                            return Err(NomErr::Error(make_error(
                                //SAFETY
                                name.borrowed().unwrap(),
                                ErrorKind::Fail
                            )));
                        }
                        //alias
                        if name == "coltype" {
                            prefix = "type".to_owned();
                        }
                    }
                }
            },
            None => {
                return Err(NomErr::Error(make_error(
                    //SAFETY
                    name.borrowed().unwrap(),
                    ErrorKind::Fail
                )));
            }
        }
    }
    let root = match root {
        Some(root) => root,
        None => return Err(NomErr::Error(make_error(input, ErrorKind::Eof)))
    };

    let key = selector
        .fold(prefix, |mut result, item| {
            if !result.is_empty() {
                result.push_str(".");
            }
            result.push_str(item.as_ref());
            result
        })
        .into_key();

    Ok(SelectResult {
        root,
        key
    })
}

/// 解析参数名
fn name(input: &str) -> ParseResult<&str> {
    //必须是字母或'#'开头
    satisfy(|c| c.is_alphabetic() || c == '#')(input)?;
    take_while1(|c: char| c.is_alphanumeric() || c == '#' || c == '_')(input)
}

/// 解析索引值
fn index(input: &str) -> ParseResult<&str> { take_while1(|c: char| c.is_numeric())(input) }

/// 查找指定`table`字段语法项
fn find_table_column<'a, 'b: 'a>(
    items: &'a Vec<ItemTableColumn<'b>>,
    name: &str
) -> Option<(usize, &'a ItemTableColumn<'b>)> {
    for (index, item) in items.iter().enumerate() {
        if let Some(v) = &item.name {
            if *v == name {
                return Some((index, item));
            }
        }
    }
    None
}

/// 查找指定`level`的分组`group`语法项
fn find_group<'a, 'b: 'a>(items: &'a Vec<Item<'b>>, level: f64) -> Option<(usize, &'a Item<'b>)> {
    for (index, item) in items.iter().enumerate() {
        if item.kind == "group" {
            if let Some(v) = item.values.get(&"level".into_key()) {
                if let Value::Number(v) = v {
                    if *v == level {
                        return Some((index, item));
                    }
                }
            }
        }
    }
    None
}

/// 查找普通语法项
fn find_item<'a, 'b: 'a>(items: &'a Vec<Item<'b>>, name: &str) -> Option<(usize, &'a Item<'b>)> {
    //通过ID查找
    if name.starts_with("#") {
        if let Ok(id) = name[1..].parse() {
            for (index, item) in items.iter().enumerate() {
                if let Some(v) = item.id {
                    if v == id {
                        return Some((index, item));
                    }
                }
            }
        }
    }
    //通过名称查找
    else {
        for (index, item) in items.iter().enumerate() {
            if let Some(v) = &item.name {
                if *v == name {
                    return Some((index, item));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        super::tests::{check_result, test_parser}, *
    };

    #[test]
    fn test_describe() {
        let dwsyn = r#"
        release 12.5;datawindow(empty_dqt=""  empty_sqt = '' num=1073741824 lit=yes  )
        header(num=564 dqt="536870912"sqt='100' )
        table(column=(type= char(80  ) updatewhereclause=yes name=col1 dbname="col1" )
        retrieve="SQL
        CLAUSE "
            arguments = ( ("a", string ), ( "b", string)  )
        )group(level=1 trailer.height=76 by= (  "col1",   'col2' ))
        group(level=2 trailer.height=76 by= (  "col1",   'col2' ))
        column(band=detail id=1 name=col1 alignment="1" tabsequence=32766 border="0" color="33554432")
        compute(band=trailer.5 alignment="2"name=compute_1 expression="count(jw_no for group 5 )+~"件~""x1="0"  )
        "#;
        let dw = test_parser(dwsyn, parse);

        #[cfg(feature = "case_insensitive")]
        let key = "DataWindow.Header.Num";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "datawindow.header.num";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::Number(564.0)));

        #[cfg(feature = "case_insensitive")]
        let key = "DataWindow.Lit";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "datawindow.lit";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::Literal("yes".into())));

        #[cfg(feature = "case_insensitive")]
        let key = "DataWindow.Trailer.1.Height";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "datawindow.trailer.1.height";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::Number(76.0)));

        #[cfg(feature = "case_insensitive")]
        let key = "DataWindow.Group.2.By";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "datawindow.group.2.by";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(
            value,
            Some(&Value::List(vec![
                Value::DoubleQuotedString("col1".into()),
                Value::SingleQuotedString("col2".into()),
            ]))
        );

        #[cfg(feature = "case_insensitive")]
        let key = "DataWindow.Table.Column.1.Type";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "datawindow.table.column.1.type";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::Literal("char(80  )".into())));

        #[cfg(feature = "case_insensitive")]
        let key = "DataWindow.Table.Column.Col1.DBName";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "datawindow.table.column.col1.dbname";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::DoubleQuotedString("col1".into())));

        #[cfg(feature = "case_insensitive")]
        let key = "DataWindow.Table.Retrieve";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "datawindow.table.retrieve";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::DoubleQuotedString("SQL\n        CLAUSE ".into())));

        #[cfg(feature = "case_insensitive")]
        let key = "Col1.color";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "col1.color";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::DoubleQuotedString("33554432".into())));

        #[cfg(feature = "case_insensitive")]
        let key = "#1.Name";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "#1.name";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::Literal("col1".into())));

        #[cfg(feature = "case_insensitive")]
        let key = "col1.DBName";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "col1.dbname";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::DoubleQuotedString("col1".into())));

        #[cfg(feature = "case_insensitive")]
        let key = "col1.ColType";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "col1.coltype";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::Literal("char(80  )".into())));

        #[cfg(feature = "case_insensitive")]
        let key = "Compute_1.Expression";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "compute_1.expression";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::DoubleQuotedString("count(jw_no for group 5 )+~\"件~\"".into())));
    }

    #[test]
    fn test_modify() {
        let dwsyn = r#"
        release 12.5;datawindow(empty_dqt=""  empty_sqt = '' num=1073741824 lit=yes  )
        header(num=564 dqt="536870912"sqt='100' )
        table(column=(type= char(80  ) updatewhereclause=yes name=col1 dbname="col1" )
        retrieve="SQL
        CLAUSE "
            arguments = ( ("a", string ), ( "b", string)  )
        )group(level=1 trailer.height=76 by= (  "col1",   'col2' ))
        group(level=2 trailer.height=76 by= (  "col1",   'col2' ))
        column(band=detail id=1 name=col1 alignment="1" tabsequence=32766 border="0" color="33554432")
        compute(band=trailer.5 alignment="2"name=compute_1 expression="count(jw_no for group 5 )+~"件~""x1="0"  )
        "#;
        let mut dw = test_parser(dwsyn, parse);

        #[cfg(feature = "case_insensitive")]
        let modifier = r"
        DataWindow.Num=12345;DataWindow.Trailer.1.Height=200
        DataWindow.Header.2.Height=200 Col1.DBName='test';
        DataWindow.Group.1.Prop='test prop'
        ";
        #[cfg(not(feature = "case_insensitive"))]
        let modifier =
            "datawindow.num=12345;datawindow.num=12345;col1.dbname='test';datawindow.group.1.level=100";
        check_result(modifier, modify(&mut dw, modifier));

        #[cfg(feature = "case_insensitive")]
        let key = "DataWindow.Num";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "datawindow.num";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::Number(12345.0)));

        //println!("{dw}");
    }
}
