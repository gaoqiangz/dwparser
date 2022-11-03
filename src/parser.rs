use crate::{ast::*, prelude::*};
use nom::{
    branch::*, bytes::complete::*, character::complete::*, combinator::*, error::{context, convert_error, make_error, ErrorKind, VerboseError}, multi::*, number::complete::*, sequence::*, Err as NomErr, IResult, Parser
};

pub type Error<'a> = NomErr<VerboseError<&'a str>>;
pub type Result<'a, T> = ::std::result::Result<T, Error<'a>>;
type ParseResult<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

#[derive(Debug, PartialEq)]
enum SumItem<'a> {
    Item(Item<'a>),
    ItemTable(ItemTable<'a>)
}

/// 解析语法
pub fn parse(input: &str) -> Result<DWSyntax> {
    let (input, (name, comment)) = srd_file_header(input)?;
    let (input, version) = version(input)?;
    let (input, (datawindow, header, summary, footer, detail, table, items)) = fold_many1(
        item,
        || {
            (
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Vec::with_capacity(2048)
            )
        },
        |(mut datawindow, mut header, mut summary, mut footer, mut detail, mut table, mut items), item| {
            match item {
                SumItem::Item(item) => {
                    match item.kind.as_ref() {
                        "datawindow" => datawindow = item.values,
                        "header" => header = item.values,
                        "summary" => summary = item.values,
                        "footer" => footer = item.values,
                        "detail" => detail = item.values,
                        _ => items.push(item)
                    }
                },
                SumItem::ItemTable(item) => {
                    table = item;
                }
            }
            (datawindow, header, summary, footer, detail, table, items)
        }
    )(input)?;
    let (input, _) = multispace0(input)?;
    if !input.is_empty() {
        return Err(NomErr::Failure(make_error(input, ErrorKind::Fail)));
    }

    Ok(DWSyntax {
        name,
        comment,
        version,
        datawindow,
        header,
        summary,
        footer,
        detail,
        table,
        items
    })
}

/// 转换友好错误信息
pub fn friendly_error(input: &str, err: Error) -> String {
    match err {
        NomErr::Error(e) | NomErr::Failure(e) => convert_error(input, e),
        _ => unreachable!()
    }
}

/// `.srd`文件头解析
///
/// # Input
///
/// ```txt
/// $PBExportHeader$dwo.srd
/// $PBExportComments$comment
/// ```
///
/// # Output
///
/// ```txt
/// (dwo.srd,comment)
/// ```
fn srd_file_header(input: &str) -> ParseResult<(Option<Cow<str>>, Option<Cow<str>>)> {
    let (input, name) = context(
        "header name",
        opt(delimited(tag("$PBExportHeader$"), take_till(|c| c == '\r'), crlf))
    )(input)?;
    let (input, comment) = context(
        "header comment",
        opt(delimited(tag("$PBExportComments$"), take_till(|c| c == '\r'), crlf))
    )(input)?;
    Ok((input, (name.map(|v| v.into()), comment.map(|v| v.into()))))
}

/// 版本号解析
///
/// # Input
///
/// ```txt
/// release 19;
/// ```
///
/// # Output
///
/// ```txt
/// 19
/// ```
fn version(input: &str) -> ParseResult<f64> {
    context(
        "version",
        delimited(
            preceded(multispace0, tag("release")),
            preceded(multispace1, double),
            preceded(multispace0, tag(";"))
        )
    )(input)
}

/// 语法项解析
///
/// # Input
///
/// ```txt
/// item(key=value key2=value)
/// ```
fn item(input: &str) -> ParseResult<SumItem> {
    /// 普通语法项解析
    ///
    /// # Input
    ///
    /// ```txt
    /// (key=value key2=value)
    /// ```
    #[inline]
    fn normal<'a>(kind: Cow<'a, str>, input: &'a str) -> ParseResult<'a, SumItem<'a>> {
        let (input, values) = value_map(input)?;
        let name = values.get("name").and_then(|v| v.as_literal()).map(|v| v.clone());
        Ok((
            input,
            SumItem::Item(Item {
                kind,
                name,
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
                            let name = values.get("name").and_then(|v| v.as_literal()).map(|v| v.clone());
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
    fn parse(input: &str) -> ParseResult<SumItem> {
        let (input, kind) = delimited(
            multispace0,
            take_while1(|c: char| c.is_alphabetic() || c == '.').map(Cow::from),
            multispace0
        )(input)?;
        if kind == "table" {
            table(input)
        } else {
            normal(kind, input)
        }
    }
    context("item", parse)(input)
}

/// 参数列表解析-MAP类型
///
/// # Input
///
/// ```txt
/// (key=value key2=value)
/// ```
///
/// # Output
///
/// ```txt
/// map<key,value>
/// ```
fn value_map(input: &str) -> ParseResult<HashMap<Cow<str>, Value>> {
    let (mut input, _) = tag("(")(input)?;
    let mut values = HashMap::with_capacity(32);
    //手写循环替代`separated_list0`,以支持松散格式
    //如:
    // key="value"key=123
    // key=char(10)key=123
    loop {
        match delimited(multispace0, key_value, multispace0)(input) {
            Ok((remaining, (key, value))) => {
                values.insert(key, value);
                input = remaining;
            },
            Err(NomErr::Error(_)) => {
                let (input, _) = tag(")")(input)?;
                return Ok((input, values));
            },
            Err(e) => return Err(e)
        }
    }
}

/// 参数解析
///
/// # Input
///
/// ```txt
/// key=value
/// ```
///
/// # Output
///
/// ```txt
/// (key,value)
/// ```
fn key_value(input: &str) -> ParseResult<(Cow<str>, Value)> {
    fn key(input: &str) -> ParseResult<Cow<str>> {
        //必须是字母开头
        satisfy(|c| c.is_alphabetic())(input)?;
        context("key", take_while1(|c: char| c.is_alphanumeric() || c == '.' || c == '_').map(Cow::from))(
            input
        )
    }
    fn value(input: &str) -> ParseResult<Value> {
        context(
            "value",
            cut(alt((value::string, value::literal, value::number, value::map, value::list, fail)))
        )(input)
    }
    separated_pair(key, delimited(multispace0, tag("="), multispace0), value)(input)
}

mod value {
    use super::*;

    /// 字面值解析
    pub fn literal(input: &str) -> ParseResult<Value> {
        /// 普通字面值
        fn normal(input: &str) -> ParseResult<&str> {
            take_while1(|c: char| c.is_alphanumeric() || c == '.' || c == '_')(input)
        }
        /// 带括号的字面值
        fn with_paren(orig_input: &str) -> ParseResult<&str> {
            use nom::{Offset, Slice};
            let (input, _) = terminated(alpha1, multispace0)(orig_input)?;
            let (input, _) =
                delimited(tag("("), delimited(multispace0, normal, multispace0), tag(")"))(input)?;
            //输出原始文本
            let offset = orig_input.offset(&input);
            Ok((input, orig_input.slice(..offset)))
        }
        //必须是字母开头
        satisfy(|c| c.is_alphabetic())(input)?;
        let parser = alt((with_paren, normal)).map(|v| Value::Literal(v.into()));
        context("literal", parser)(input)
    }

    /// 字符串解析
    pub fn string(input: &str) -> ParseResult<Value> {
        /// 不同引号字符串(`""`/`''`)转义处理
        fn quoted(qot: char) -> impl Fn(&str) -> ParseResult<&str> {
            move |input: &str| {
                delimited(
                    char(qot),
                    escaped(
                        take_till1(|c: char| c == '~' || c == qot),
                        '~',
                        anychar //PB字符串转义不限制字符
                    ),
                    char(qot)
                )(input)
            }
        }
        //NOTE
        //`escaped`要求`normal`参数失败才会匹配`control_char`,所以必须使用`take_till1`组合子
        //因此需要单独匹配空字符串: `""`/`''`
        let double_quoted =
            alt((quoted('"'), tag("\"\"").map(|_| ""))).map(|v| Value::DoubleQuotedString(v.into()));
        let single_quoted =
            alt((quoted('\''), tag("''").map(|_| ""))).map(|v| Value::SingleQuotedString(v.into()));
        let parser = alt((double_quoted, single_quoted));
        context("string", parser)(input)
    }

    /// 数值解析
    pub fn number(input: &str) -> ParseResult<Value> { context("number", double.map(Value::Number))(input) }

    /// 多值列表解析
    pub fn list(input: &str) -> ParseResult<Value> {
        let parser = delimited(
            tag("("),
            separated_list0(
                tag(","),
                delimited(multispace0, cut(alt((string, literal, list, fail))), multispace0)
            )
            .map(Value::List),
            tag(")")
        );
        context("list", parser)(input)
    }

    /// Key-Value值列表解析
    pub fn map(input: &str) -> ParseResult<Value> {
        let parser = value_map.map(Value::Map);
        context("map", parser)(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_parser<'a, F, O>(input: &'a str, f: F) -> O
    where
        F: Fn(&'a str) -> Result<'a, O>
    {
        match f(input) {
            Ok(v) => v,
            Err(e) => panic!("{}", friendly_error(input, e))
        }
    }

    #[test]
    fn test_srd_file_header() {
        let (input, output) =
            test_parser("$PBExportHeader$d_rpt_jewel_loc_in_cost_list.srd\r\n", srd_file_header);
        assert_eq!(input, "");
        assert_eq!(output, (Some(Cow::from("d_rpt_jewel_loc_in_cost_list.srd")), None));
        let (input, output) = test_parser("$PBExportComments$首饰入库成本明细表\r\n", srd_file_header);
        assert_eq!(input, "");
        assert_eq!(output, (None, Some(Cow::from("首饰入库成本明细表"))));
        let (input, output) = test_parser(
            "$PBExportHeader$d_rpt_jewel_loc_in_cost_list.srd\r\n$PBExportComments$首饰入库成本明细表\r\nrelease 19;",
            srd_file_header
        );
        assert_eq!(input, "release 19;");
        assert_eq!(
            output,
            (Some(Cow::from("d_rpt_jewel_loc_in_cost_list.srd")), Some(Cow::from("首饰入库成本明细表")))
        );
        let (input, output) = test_parser("WrongHeader", srd_file_header);
        assert_eq!(input, "WrongHeader");
        assert_eq!(output, (None, None));
    }

    #[test]
    fn test_version() {
        let (input, output) = test_parser("release 19;", version);
        assert_eq!(input, "");
        assert_eq!(output, 19.);
        let (input, output) = test_parser(" release   12.5  ;datawindow()", version);
        assert_eq!(input, "datawindow()");
        assert_eq!(output, 12.5);
        assert!(version("releasex 19;").is_err());
        assert!(version("release 19 . 5;").is_err());
    }

    #[test]
    fn test_item() {
        let (input, output) = test_parser("group(key=value key2=132 key3='abc~'123\"')", item);
        assert_eq!(input, "");
        assert_eq!(
            output,
            SumItem::Item(Item {
                kind: "group".into(),
                name: None,
                values: HashMap::from([
                    ("key".into(), Value::Literal("value".into())),
                    ("key2".into(), Value::Number(132.)),
                    ("key3".into(), Value::SingleQuotedString("abc~'123\"".into())),
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
                        name: Some("col1".into()),
                        values: HashMap::from([
                            ("type".into(), Value::Literal("char(10)".into())),
                            ("name".into(), Value::Literal("col1".into())),
                        ])
                    },
                    ItemTableColumn {
                        name: Some("col2".into()),
                        values: HashMap::from([
                            ("type".into(), Value::Literal("char(20)".into())),
                            ("name".into(), Value::Literal("col2".into())),
                        ])
                    }
                ],
                values: HashMap::from([(
                    "arguments".into(),
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
    fn test_value_map() {
        let (input, output) = test_parser("(key1=123.45 key2=value key3='abc'key4=\"abc\")", value_map);
        assert_eq!(input, "");
        assert_eq!(
            output,
            HashMap::from([
                ("key1".into(), Value::Number(123.45)),
                ("key2".into(), Value::Literal("value".into())),
                ("key3".into(), Value::SingleQuotedString("abc".into())),
                ("key4".into(), Value::DoubleQuotedString("abc".into())),
            ])
        );
        assert!(value_map("(123=bac)").is_err());
        assert!(value_map("(key1=123 2key2=345)").is_err());
    }

    #[test]
    fn test_key_value() {
        let (input, output) = test_parser("key=123.45", key_value);
        assert_eq!(input, "");
        assert_eq!(output, ("key".into(), Value::Number(123.45)));
        let (input, output) = test_parser("key=value", key_value);
        assert_eq!(input, "");
        assert_eq!(output, ("key".into(), Value::Literal("value".into())));
        let (input, output) = test_parser("key='abc~'def~'\nghi'", key_value);
        assert_eq!(input, "");
        assert_eq!(output, ("key".into(), Value::SingleQuotedString("abc~'def~'\nghi".into())));
        let (input, output) = test_parser("key=\"abc'\ndef~\"ghi~\"\"", key_value);
        assert_eq!(input, "");
        assert_eq!(output, ("key".into(), Value::DoubleQuotedString("abc'\ndef~\"ghi~\"".into())));
        assert!(key_value("123=bac").is_err());
        assert!(key_value("2key2=345").is_err());
        assert!(key_value("123:sadf23").is_err());
    }

    #[test]
    fn test_parse() {
        let dwsyn = r#"
        release 12.5;datawindow(empty_dqt=""  empty_sqt = '' num=1073741824 lit=yes  )
        header(num=564 dqt="536870912"sqt='100' )
        table(column=(type= char(80  ) updatewhereclause=yes name=col1 dbname="col1" )
        retrieve="SQL
        CLAUSE "
            arguments = ( ("a", string ), ( "b", string)  )
        )group(trailer.height=76 by= (  "col1",   'col2' ))
        compute(band=trailer.5 alignment="2"name=compute_1 expression="count(jw_no for group 5 )+~"件~""x1="0"  )
        "#;
        let dw = test_parser(dwsyn, parse);
        assert_eq!(dw, DWSyntax {
            name: None,
            comment: None,
            version: 12.5,
            datawindow: HashMap::from([
                ("empty_dqt".into(), Value::DoubleQuotedString("".into())),
                ("empty_sqt".into(), Value::SingleQuotedString("".into())),
                ("num".into(), Value::Number(1073741824.)),
                ("lit".into(), Value::Literal("yes".into()))
            ]),
            header: HashMap::from([
                ("num".into(), Value::Number(564.)),
                ("dqt".into(), Value::DoubleQuotedString("536870912".into())),
                ("sqt".into(), Value::SingleQuotedString("100".into()))
            ]),
            summary: Default::default(),
            footer: Default::default(),
            detail: Default::default(),
            table: ItemTable {
                columns: vec![ItemTableColumn {
                    name: Some("col1".into()),
                    values: HashMap::from([
                        ("type".into(), Value::Literal("char(80  )".into())),
                        ("updatewhereclause".into(), Value::Literal("yes".into())),
                        ("name".into(), Value::Literal("col1".into())),
                        ("dbname".into(), Value::DoubleQuotedString("col1".into())),
                    ])
                }],
                values: HashMap::from([
                    ("retrieve".into(), Value::DoubleQuotedString("SQL\n        CLAUSE ".into())),
                    (
                        "arguments".into(),
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
                    ),
                ])
            },
            items: vec![
                Item {
                    kind: "group".into(),
                    name: None,
                    values: HashMap::from([
                        ("trailer.height".into(), Value::Number(76.)),
                        (
                            "by".into(),
                            Value::List(vec![
                                Value::DoubleQuotedString("col1".into()),
                                Value::SingleQuotedString("col2".into()),
                            ])
                        ),
                    ])
                },
                Item {
                    kind: "compute".into(),
                    name: Some("compute_1".into()),
                    values: HashMap::from([
                        ("band".into(), Value::Literal("trailer.5".into())),
                        ("alignment".into(), Value::DoubleQuotedString("2".into())),
                        ("name".into(), Value::Literal("compute_1".into())),
                        (
                            "expression".into(),
                            Value::DoubleQuotedString(r#"count(jw_no for group 5 )+~"件~""#.into())
                        ),
                        ("x1".into(), Value::DoubleQuotedString("0".into()))
                    ])
                }
            ]
        });
        assert_eq!(
            dw.to_string(),
            "release 12.5;\r\ndatawindow(empty_dqt=\"\" empty_sqt='' num=1073741824 lit=yes)\r\nheader(num=564 dqt=\"536870912\" sqt='100')\r\ntable(column=(type=char(80  ) updatewhereclause=yes name=col1 dbname=\"col1\")\r\nretrieve=\"SQL\n        CLAUSE \" arguments=((\"a\", string), (\"b\", string)))\r\ngroup(trailer.height=76 by=(\"col1\", 'col2'))\r\ncompute(band=trailer.5 alignment=\"2\" name=compute_1 expression=\"count(jw_no for group 5 )+~\"件~\"\" x1=\"0\")\r\n"
        );
    }
}
