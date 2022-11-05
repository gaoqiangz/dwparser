use super::*;
use std::mem::transmute;

/// 获取指定语法项的参数值
pub fn describe<'a, 'b: 'a, 'c>(syn: &'a DWSyntax<'b>, input: &'c str) -> Result<'c, Option<&'a Value<'b>>> {
    fn name(input: &str) -> ParseResult<&str> {
        //必须是字母开头
        satisfy(|c| c.is_alphabetic())(input)?;
        take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)
    }
    fn number(input: &str) -> ParseResult<&str> { take_while1(|c: char| c.is_numeric())(input) }
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
    fn find_column<'a, 'b: 'a>(
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

    //解析选择语法
    let (_, selector) = terminated(
        preceded(multispace0, separated_list1(tag("."), alt((name, number)).map(IntoKey::into_key))),
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
        if name == "datawindow" {
            //TODO
            //- tree
            let name = selector.next().unwrap();
            if name == "header" {
                let mut values = &syn.header;
                //header.<group #>.prop
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
                //footer.<group #>.prop
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
                //trailer.<group #>.prop
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
                //group.<group #>.prop
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
                //- column.<column #>.prop
                //- column.<column name>.prop
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
                        } else {
                            if let Some(item) = find_column(&syn.table.columns, name.as_ref()) {
                                values = &item.values;
                            }
                        }
                    }
                }
                values
            } else {
                prefix = name.as_ref().to_owned();
                &syn.datawindow
            }
        } else {
            match syn.items.iter().find(|item| {
                if let Some(n) = &item.name {
                    *n == name
                } else {
                    false
                }
            }) {
                Some(item) => &item.values,
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

    let selector = selector.fold(prefix, |mut result, item| {
        if !result.is_empty() {
            result.push_str(".");
        }
        result.push_str(item.as_ref());
        result
    });

    Ok(values.get(&selector.into_key()))
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
        let key = "Compute_1.Expression";
        #[cfg(not(feature = "case_insensitive"))]
        let key = "compute_1.expression";
        let value = check_result(key, describe(&dw, key));
        assert_eq!(value, Some(&Value::DoubleQuotedString("count(jw_no for group 5 )+~\"件~\"".into())));
    }
}
