use super::*;

#[derive(Debug, PartialEq)]
pub enum SumItem<'a> {
    Item(Item<'a>),
    ItemTable(ItemTable<'a>),
    ItemData(Vec<Value<'a>>)
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
    Ok((
        input,
        SumItem::Item(Item {
            kind,
            name,
            id,
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
    use nom::{Offset,Slice};
    fn null(input:&str)->ParseResult<Value>{
        delimited(multispace0, tag("null").map(|v|{Value::Literal(Cow::from(v))}), multispace0)(input)
    }
    fn date(input: &str) -> ParseResult<Value> {
        let (i, _) = 
            delimited(delimited(digit1, tag("-"), digit1), tag("-"), digit1)
            (input)?;
        let ofs = input.offset(i);
        Ok((input.slice(ofs..), Value::Literal(input.slice(..ofs).into())))
    }
    fn time(input: &str) -> ParseResult<Value> {
        let (i, _) = 
            delimited(
                delimited(delimited(digit1, tag(":"), digit1), tag(":"), digit1),
                tag(":"),
                digit1,
            )(input)?;
        let ofs = input.offset(i);
        Ok((input.slice(ofs..), Value::Literal(input.slice(..ofs).into())))
    }
    fn datetime(input: &str) -> ParseResult<Value> {
        let (i, _) = delimited(date, multispace1, time)(input)?;
        let ofs = input.offset(i);
        Ok((input.slice(ofs..), Value::Literal(input.slice(..ofs).into())))
    }
    fn parser_dt(input:&str)->ParseResult<Value>{
        context("datetime",alt((datetime,date,time)))(input)
    }
    fn last_dot(input:&str)->ParseResult<Value>{
        Ok((input,Value::List(vec![])))
    }
    fn parser_null(input:&str)->ParseResult<Value>{
        let mut rt_vec:Vec<Value> = vec![];
        let (mut input,output) = null(input)?;
        rt_vec.push(output);
        loop {
            match null(input) {
                Ok((i,o))=>{
                    input = i;
                    rt_vec.push(o);                       
                },
                Err(_)=>{
                    if let Ok((input,o)) = alt((parser_dt,value::string,value::number,fail))(input)
                    {
                        rt_vec.push(o);
                        return Ok((input,Value::List(rt_vec)));
                    }
                    break;
                }
            }
        }
        Ok((input,Value::List(rt_vec)))
    }    
    let mut parser = delimited(
        tag("("),
        separated_list0(tag(","),delimited(multispace0, alt((parser_null,parser_dt,value::string,value::number,last_dot,fail)),multispace0 ) )
        .map(|v:Vec<Value>|{
            let mut rv = vec![];
            for item in v{
                match item {
                    Value::List(_item)=>{
                        for _v in _item{
                            rv.push(_v);
                        }
                    },
                    _=>{
                        rv.push(item);
                    }
                }
            };
            rv
        }),
        tag(")"),
    );
    let (input, value) = parser(input)?;
    Ok((input, SumItem::ItemData(value)))
}

#[cfg(test)]
mod tests {
    use super::{super::tests::test_parser, *};

    #[test]
    fn test_item() {
        let (input, output) = test_parser("group(key=value key2=132 key3='abc~'123\"')", item);
        assert_eq!(input, "");
        assert_eq!(
            output,
            SumItem::Item(Item {
                kind: "group".into_key(),
                name: None,
                id: None,
                values: HashMap::from([
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
        let dw = r#"data(null null 0  ,"自增(ID)","\r\n参数1",1,2,3,null "固定,字符","参数2",null 1,null  1 ,null ,  )abc"#;
        let (input, output) = test_parser(dw, item);
        assert_eq!(input, "abc");
        assert_eq!(
            output,
            SumItem::ItemData(vec![
                Value::Literal(Cow::from("null")),
                Value::Literal(Cow::from("null")),
                Value::Number(0.0),
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
                Value::Number(1.0),
                Value::Literal(Cow::from("null")),
            ])
        );
    }
}
