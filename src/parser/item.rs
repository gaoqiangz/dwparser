use super::*;

#[derive(Debug, PartialEq)]
pub enum SumItem<'a> {
    Item(Item<'a>),
    ItemTable(ItemTable<'a>),
    ItemData(Vec<Value<'a>>)
}

impl<'a> SumItem<'a> {
    /// 拷贝值并协变为目标生命期
    pub(crate) fn to_owned<'r>(&self) -> SumItem<'r> {
        match self {
            SumItem::Item(item) => SumItem::Item(item.to_owned()),
            SumItem::ItemTable(item) => SumItem::ItemTable(item.to_owned()),
            SumItem::ItemData(item) => SumItem::ItemData(item.iter().map(|v| v.to_owned()).collect())
        }
    }
}

/// 语法项解析
///
/// # Input
///
/// ```txt
/// item(key=value key2=value)
/// ```
pub fn item(input: &str) -> ParseResult<SumItem> {
    fn parse(input: &str) -> ParseResult<SumItem> {
        let (input, kind) = delimited(
            multispace0,
            take_while1(|c: char| c.is_alphabetic() || c == '.').map(IntoKey::into_key),
            multispace0
        )(input)?;
        if kind == "table" {
            table(input)
        } else if kind == "data" {
            data(input)
        } else {
            normal(kind, input)
        }
    }
    context("item", parse)(input)
}

/// 普通语法项解析
///
/// # Input
///
/// ```txt
/// (key=value key2=value)
/// ```
#[inline]
fn normal<'a>(kind: Key<'a>, input: &'a str) -> ParseResult<'a, SumItem<'a>> {
    let (input, values) = value_map(input)?;
    let name = values.get(&"name".into_key()).and_then(|v| v.as_literal()).map(|v| v.clone().into_key());
    let id = values.get(&"id".into_key()).and_then(|v| v.as_number()).map(|v| v as u32);
    let level = values.get(&"level".into_key()).and_then(|v| v.as_number()).map(|v| v as u32);
    Ok((
        input,
        SumItem::Item(Item {
            kind,
            name,
            id,
            level,
            values
        })
    ))
}

/// `table`语法项解析
///
/// # Input
///
/// ```txt
/// (column=(type=type) column=(type=type) key=value key=value)
/// ```
#[inline]
fn table(input: &str) -> ParseResult<SumItem> {
    let (mut input, _) = tag("(")(input)?;
    let mut columns = Vec::with_capacity(64);
    let mut values = HashMap::with_capacity(8);
    //手写循环替代`separated_list0`,以支持松散格式
    //如:
    // key="value"key=123
    // key=char(10)key=123
    loop {
        match delimited(multispace0, key_value, multispace0)(input) {
            Ok((remaining, (key, value))) => {
                if key == "column" {
                    if let Value::Map(values) = value {
                        let name = values
                            .get(&"name".into_key())
                            .and_then(|v| v.as_literal())
                            .map(|v| v.clone().into_key());
                        columns.push(ItemTableColumn {
                            name,
                            values
                        })
                    } else {
                        return Err(NomErr::Error(make_error(input, ErrorKind::Fail)));
                    }
                } else {
                    values.insert(key, value);
                }
                input = remaining;
            },
            Err(NomErr::Error(_)) => {
                let (input, _) = tag(")")(input)?;
                return Ok((
                    input,
                    SumItem::ItemTable(ItemTable {
                        columns,
                        values
                    })
                ));
            },
            Err(e) => return Err(e)
        }
    }
}

/// `data`语法项解析
///
/// # Input
///
/// ```txt
/// data(val, null val, val)
/// ```
#[inline]
fn data(input: &str) -> ParseResult<SumItem> {
    //yyyy-mm-dd
    fn date(input: &str) -> ParseResult<Value> {
        recognize(tuple((
            map_parser(digit1, take(4usize)),
            tag("-"),
            map_parser(digit1, take(2usize)),
            tag("-"),
            map_parser(digit1, take(2usize))
        )))
        .map(|v: &str| Value::Literal(v.into()))
        .parse(input)
    }
    //hh:mm:ss:ssss
    fn time(input: &str) -> ParseResult<Value> {
        recognize(tuple((
            map_parser(digit1, take(2usize)),
            tag(":"),
            map_parser(digit1, take(2usize)),
            tag(":"),
            map_parser(digit1, take(2usize)),
            tag(":"),
            map_parser(digit1, take(4usize))
        )))
        .map(|v: &str| Value::Literal(v.into()))
        .parse(input)
    }
    //yyyy-mm-dd hh:mm:ss:ssss
    fn datetime(input: &str) -> ParseResult<Value> {
        recognize(tuple((date, tag(" "), time))).map(|v: &str| Value::Literal(v.into())).parse(input)
    }
    //null
    fn null(input: &str) -> ParseResult<Value> {
        tag("null").map(|v: &str| Value::Literal(v.into())).parse(input)
    }
    //空格分隔的值列表
    fn list(input: &str) -> ParseResult<Value> {
        separated_list0(multispace1, alt((null, value::string, datetime, date, time, value::number)))
            .map(Value::List)
            .parse(input)
    }
    let parser = delimited(
        tag("("),
        separated_list0(
            tag(","),
            delimited(
                multispace0,
                //NOTE `list`可消除最后可选的(`,`),因为`separated_list0`始终会成功
                alt((value::string, datetime, date, time, value::number, list)),
                multispace0
            )
        ),
        tag(")")
    );
    let mut parser = parser.map(|list| {
        let mut rv = vec![];
        for item in list {
            match item {
                Value::List(item) => rv.extend(item),
                _ => {
                    rv.push(item);
                }
            }
        }
        rv
    });
    let (input, value) = parser.parse(input)?;
    Ok((input, SumItem::ItemData(value)))
}

#[cfg(test)]
mod tests {
    use super::{super::tests::test_parser, *};

    #[test]
    fn test_item() {
        let (input, output) = test_parser("group(level=1 key=value key2=132 key3='abc~'123\"')", item);
        assert_eq!(input, "");
        assert_eq!(
            output,
            SumItem::Item(Item {
                kind: "group".into_key(),
                name: None,
                id: None,
                level: Some(1),
                values: HashMap::from([
                    ("level".into_key(), Value::Number(1.)),
                    ("key".into_key(), Value::Literal("value".into())),
                    ("key2".into_key(), Value::Number(132.)),
                    ("key3".into_key(), Value::SingleQuotedString("abc~'123\"".into())),
                ])
            })
        );
        let (input, output) = test_parser(
            "table(column=(type=char(10)name=col1) column=(type=char(20)name=col2)arguments = ( (\"a\", string ), ( \"b\", string)  ))",
            item
        );
        assert_eq!(input, "");
        assert_eq!(
            output,
            SumItem::ItemTable(ItemTable {
                columns: vec![
                    ItemTableColumn {
                        name: Some("col1".into_key()),
                        values: HashMap::from([
                            ("type".into_key(), Value::Literal("char(10)".into())),
                            ("name".into_key(), Value::Literal("col1".into())),
                        ])
                    },
                    ItemTableColumn {
                        name: Some("col2".into_key()),
                        values: HashMap::from([
                            ("type".into_key(), Value::Literal("char(20)".into())),
                            ("name".into_key(), Value::Literal("col2".into())),
                        ])
                    }
                ],
                values: HashMap::from([(
                    "arguments".into_key(),
                    Value::List(vec![
                        Value::List(vec![
                            Value::DoubleQuotedString("a".into()),
                            Value::Literal("string".into())
                        ]),
                        Value::List(vec![
                            Value::DoubleQuotedString("b".into()),
                            Value::Literal("string".into())
                        ])
                    ])
                ),])
            })
        );
        assert!(item("group[]").is_err());
        assert!(item("group xx()").is_err());
    }

    #[test]
    fn test_item_data() {
        let dw = r#"data( 1, null  2001-12-31 12:00:12:0000,2001-12-31,11:11:23:0000  ,"自增(ID)","\r\n参数1",1,2,3,null "固定,字符","参数2",null 1,null, )abc"#;
        let (input, output) = test_parser(dw, item);
        assert_eq!(input, "abc");
        assert_eq!(
            output,
            SumItem::ItemData(vec![
                Value::Number(1.0),
                Value::Literal(Cow::from("null")),
                Value::Literal(Cow::from("2001-12-31 12:00:12:0000")),
                Value::Literal(Cow::from("2001-12-31")),
                Value::Literal(Cow::from("11:11:23:0000")),
                Value::DoubleQuotedString(Cow::from("自增(ID)")),
                Value::DoubleQuotedString(Cow::from("\\r\\n参数1")),
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0),
                Value::Literal(Cow::from("null")),
                Value::DoubleQuotedString(Cow::from("固定,字符")),
                Value::DoubleQuotedString(Cow::from("参数2")),
                Value::Literal(Cow::from("null")),
                Value::Number(1.0),
                Value::Literal(Cow::from("null")),
            ])
        );
    }
}
