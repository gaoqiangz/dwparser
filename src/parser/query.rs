use super::*;
use std::mem::transmute;

/// 获取指定语法项的参数值
///
/// 兼容`DataWindow::Describe`参数
pub fn describe<'a, 'b: 'a, 'c>(syn: &'a DWSyntax<'b>, input: &'c str) -> Result<'c, Option<&'a Value<'b>>> {
    match select(syn, input)? {
        SelectResult::Value(v) => Ok(Some(v)),
        _ => Ok(None)
    }
}

/// 修改语法项的参数值
///
/// 兼容`DataWindow::Modify`参数
pub fn modify<'a, 'b: 'a, 'c>(syn: &'a mut DWSyntax<'b>, input: &'c str) -> Result<'c, ()> {
    enum ModifyKind<'a> {
        Assign(&'a str, Value<'a>),
        Create(SumItem<'a>),
        Destroy(KeyType<'a>)
    }
    fn assign(input: &str) -> ParseResult<ModifyKind> {
        fn key(input: &str) -> ParseResult<&str> {
            take_while1(|c: char| c.is_alphanumeric() || c == '#' || c == '.' || c == '_')(input)
        }
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
        name.map(|v| ModifyKind::Destroy(v.into_key())).parse(input)
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
                match select(syn, selector)? {
                    SelectResult::Value(v) => {
                        //SAFETY
                        //转换为mutable引用
                        let v: &'a mut Value<'b> = unsafe {
                            let v = v as *const Value;
                            let v = v as *mut Value;
                            &mut *v
                        };
                        *v = value.to_static();
                    },
                    SelectResult::Map(map, key) => {
                        //SAFETY
                        //转换为mutable引用
                        let map: &'a mut HashMap<KeyType<'b>, Value<'b>> = unsafe {
                            let map = map as *const HashMap<KeyType, Value>;
                            let map = map as *mut HashMap<KeyType, Value>;
                            &mut *map
                        };
                        map.insert(Cow::clone(&key).into_owned().into_key(), value.to_static());
                    }
                }
            },
            ModifyKind::Create(item) => todo!(),
            ModifyKind::Destroy(name) => todo!()
        }
    }
    Ok(())
}

/// 选取结果
enum SelectResult<'a, 'b: 'a, 'c> {
    /// 选择到值
    Value(&'a Value<'b>),
    /// 未选取到值
    Map(&'a HashMap<KeyType<'b>, Value<'b>>, KeyType<'c>)
}

/// 选择指定语法项的参数项
fn select<'a, 'b: 'a, 'c>(syn: &'a DWSyntax<'b>, input: &'c str) -> Result<'c, SelectResult<'a, 'b, 'c>> {
    //解析选择语法
    let (_, selector) = terminated(
        preceded(multispace0, separated_list1(tag("."), alt((name, index)).map(IntoKey::into_key))),
        terminated(multispace0, eof)
    )(input)?;
    if selector.len() < 2 {
        return Err(NomErr::Error(make_error(input, ErrorKind::Eof)));
    }
    let mut selector = selector.into_iter();
    let mut prefix = String::new();

    //选取参数列表
    let values = {
        let name = selector.next().unwrap();
        //datawindow名称下的语法项
        if name == "datawindow" {
            //TODO
            //- tree
            let name = selector.next().unwrap();
            if name == "header" {
                let mut values = &syn.header;
                //datawindow.header.<group #>.prop
                if selector.len() >= 2 {
                    let name = selector.next().unwrap();
                    if let Ok(level) = name.parse() {
                        if let Some(item) = find_group(&syn.items, level) {
                            prefix = "header".to_owned();
                            values = &item.values;
                        } else {
                            return Err(NomErr::Error(make_error(
                                //SAFETY
                                unsafe { transmute(name.as_ref()) },
                                ErrorKind::Fail
                            )));
                        }
                    }
                }
                values
            } else if name == "footer" {
                let mut values = &syn.footer;
                //datawindow.footer.<group #>.prop
                if selector.len() >= 2 {
                    let name = selector.next().unwrap();
                    if let Ok(level) = name.parse() {
                        if let Some(item) = find_group(&syn.items, level) {
                            prefix = "footer".to_owned();
                            values = &item.values;
                        } else {
                            return Err(NomErr::Error(make_error(
                                //SAFETY
                                unsafe { transmute(name.as_ref()) },
                                ErrorKind::Fail
                            )));
                        }
                    }
                }
                values
            } else if name == "trailer" {
                let mut values = None;
                //datawindow.trailer.<group #>.prop
                if selector.len() >= 2 {
                    if let Ok(level) = selector.next().unwrap().parse() {
                        if let Some(item) = find_group(&syn.items, level) {
                            prefix = "trailer".to_owned();
                            values = Some(&item.values);
                        }
                    }
                }
                match values {
                    Some(values) => values,
                    None => {
                        return Err(NomErr::Error(make_error(
                            //SAFETY
                            unsafe { transmute(name.as_ref()) },
                            ErrorKind::Fail
                        )));
                    }
                }
            } else if name == "group" {
                let mut values = None;
                //datawindow.group.<group #>.prop
                if selector.len() >= 2 {
                    if let Ok(level) = selector.next().unwrap().parse() {
                        if let Some(item) = find_group(&syn.items, level) {
                            values = Some(&item.values);
                        }
                    }
                }
                match values {
                    Some(values) => values,
                    None => {
                        return Err(NomErr::Error(make_error(
                            //SAFETY
                            unsafe { transmute(name.as_ref()) },
                            ErrorKind::Fail
                        )));
                    }
                }
            } else if name == "summary" {
                &syn.summary
            } else if name == "detail" {
                &syn.detail
            } else if name == "table" {
                let mut values = &syn.table.values;
                //- datawindow.table.column.<column #>.prop
                //- datawindow.table.column.<column name>.prop
                if selector.len() >= 3 {
                    let name = selector.next().unwrap();
                    if name == "column" {
                        let name = selector.next().unwrap();
                        if let Ok(idx) = name.parse::<usize>() {
                            if idx > 0 && idx <= syn.table.columns.len() {
                                values = &syn.table.columns[idx - 1].values;
                            } else {
                                return Err(NomErr::Error(make_error(
                                    //SAFETY
                                    unsafe { transmute(name.as_ref()) },
                                    ErrorKind::Fail
                                )));
                            }
                        } else if let Some(item) = find_table_column(&syn.table.columns, name.as_ref()) {
                            values = &item.values;
                        }
                    }
                }
                values
            } else {
                prefix = name.as_ref().to_owned();
                &syn.datawindow
            }
        }
        //具名的普通语法项
        else {
            match find_item(&syn.items, name.as_ref()) {
                Some(item) => {
                    let mut values = &item.values;
                    //特殊处理字段属性
                    //- col.coltype
                    //- col.dbname
                    if selector.len() == 1 && item.kind == "column" && item.id.is_some() {
                        let name = selector.next().unwrap();
                        prefix = name.as_ref().to_owned();
                        if name == "coltype" || name == "dbname" {
                            let idx = item.id.unwrap() as usize;
                            if idx > 0 && idx <= syn.table.columns.len() {
                                values = &syn.table.columns[idx - 1].values;
                            } else {
                                return Err(NomErr::Error(make_error(
                                    //SAFETY
                                    unsafe { transmute(name.as_ref()) },
                                    ErrorKind::Fail
                                )));
                            }
                            //更名
                            if name == "coltype" {
                                prefix = "type".to_owned();
                            }
                        }
                    }
                    values
                },
                None => {
                    return Err(NomErr::Error(make_error(
                        //SAFETY
                        unsafe { transmute(name.as_ref()) },
                        ErrorKind::Fail
                    )));
                }
            }
        }
    };

    let selector = selector
        .fold(prefix, |mut result, item| {
            if !result.is_empty() {
                result.push_str(".");
            }
            result.push_str(item.as_ref());
            result
        })
        .into_key();

    match values.get(&selector) {
        Some(v) => Ok(SelectResult::Value(v)),
        None => Ok(SelectResult::Map(values, selector))
    }
}

/// 解析参数名
fn name(input: &str) -> ParseResult<&str> {
    //必须是字母或'#'开头
    satisfy(|c| c.is_alphabetic() || c == '#')(input)?;
    take_while1(|c: char| c.is_alphanumeric() || c == '#' || c == '_')(input)
}

/// 解析索引值
fn index(input: &str) -> ParseResult<&str> { take_while1(|c: char| c.is_numeric())(input) }

/// 查找指定`level`的分组`group`语法项
fn find_group<'a, 'b: 'a>(items: &'a Vec<Item<'b>>, level: f64) -> Option<&'a Item<'b>> {
    for item in items {
        if item.kind == "group" {
            if let Some(v) = item.values.get(&"level".into_key()) {
                if let Value::Number(v) = v {
                    if *v == level {
                        return Some(item);
                    }
                }
            }
        }
    }
    None
}

/// 查找指定`table`字段语法项
fn find_table_column<'a, 'b: 'a>(
    items: &'a Vec<ItemTableColumn<'b>>,
    name: &str
) -> Option<&'a ItemTableColumn<'b>> {
    for item in items {
        if let Some(v) = &item.name {
            if *v == name {
                return Some(item);
            }
        }
    }
    None
}

/// 查找普通语法项
fn find_item<'a, 'b: 'a>(items: &'a Vec<Item<'b>>, name: &str) -> Option<&'a Item<'b>> {
    //通过ID查找
    if name.starts_with("#") {
        if let Ok(id) = name[1..].parse() {
            for item in items {
                if let Some(v) = item.id {
                    if v == id {
                        return Some(item);
                    }
                }
            }
        }
    }
    //通过名称查找
    else {
        for item in items {
            if let Some(v) = &item.name {
                if *v == name {
                    return Some(item);
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
            "datawindow.num=12345;DataWindow.Num=12345;col1.dbname='test';datawindow.group.1.level=100";
        check_result(modifier, modify(&mut dw, modifier));

        #[cfg(feature = "case_insensitive")]
        let key = "DataWindow.Num";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "datawindow.num";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::Number(12345.0)));

        println!("{dw}");
    }
}
