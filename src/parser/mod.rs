use crate::{ast::*, prelude::*};
use nom::{
    branch::*, bytes::complete::*, character::complete::*, combinator::*, error::{context, convert_error, make_error, ErrorKind, VerboseError}, multi::*, number::complete::*, sequence::*, Err as NomErr, IResult, Parser
};

mod item;
mod value;
#[cfg(feature = "query")]
pub mod query;

use item::{item, SumItem};

pub type Error<'a> = NomErr<VerboseError<&'a str>>;
pub type Result<'a, T> = ::std::result::Result<T, Error<'a>>;
type ParseResult<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

/// 解析语法
pub fn parse(input: &str) -> Result<DWSyntax> {
    let (input, (name, comment)) = srd_file_header(input)?;
    let (input, version) = version(input)?;
    let (input, (datawindow, header, summary, footer, detail, table, data, items)) = fold_many1(
        item,
        || {
            (
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Vec::with_capacity(2048)
            )
        },
        |(
            mut datawindow,
            mut header,
            mut summary,
            mut footer,
            mut detail,
            mut table,
            mut data,
            mut items
        ),
         item| {
            match item {
                SumItem::Item(item) => {
                    if item.kind == "datawindow" {
                        datawindow = item.values;
                    } else if item.kind == "header" {
                        header = item.values;
                    } else if item.kind == "summary" {
                        summary = item.values;
                    } else if item.kind == "footer" {
                        footer = item.values;
                    } else if item.kind == "detail" {
                        detail = item.values;
                    } else {
                        items.push(item);
                    }
                },
                SumItem::ItemTable(item) => {
                    table = item;
                },
                SumItem::ItemData(item) => {
                    data = item;
                }
            }
            (datawindow, header, summary, footer, detail, table, data, items)
        }
    )(input)?;
    terminated(multispace0, eof)(input)?;

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
        data,
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
    let (input, name) =
        context("header name", opt(delimited(tag("$PBExportHeader$"), is_not("\r"), crlf)))(input)?;
    let (input, comment) =
        context("header comment", opt(delimited(tag("$PBExportComments$"), is_not("\r"), crlf)))(input)?;
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
fn value_map(input: &str) -> ParseResult<HashMap<Key, Value>> {
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
fn key_value(input: &str) -> ParseResult<(Key, Value)> {
    fn key(input: &str) -> ParseResult<Key> {
        //必须是字母开头
        satisfy(|c| c.is_alphabetic())(input)?;
        context(
            "key",
            take_while1(|c: char| c.is_alphanumeric() || c == '.' || c == '_').map(IntoKey::into_key)
        )(input)
    }
    fn value(input: &str) -> ParseResult<Value> {
        context(
            "value",
            cut(alt((value::string, value::literal, value::number, value::map, value::list, fail)))
        )(input)
    }
    separated_pair(key, delimited(multispace0, tag("="), multispace0), value)(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn test_parser<'a, F, O>(input: &'a str, f: F) -> O
    where
        F: Fn(&'a str) -> Result<'a, O>
    {
        check_result(input, f(input))
    }

    pub fn check_result<'a, O>(input: &'a str, rv: Result<'a, O>) -> O {
        match rv {
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
    fn test_value_map() {
        let (input, output) = test_parser("(key1=123.45 key2=value key3='abc'key4=\"abc\")", value_map);
        assert_eq!(input, "");
        assert_eq!(
            output,
            HashMap::from([
                ("key1".into_key(), Value::Number(123.45)),
                ("key2".into_key(), Value::Literal("value".into())),
                ("key3".into_key(), Value::SingleQuotedString("abc".into())),
                ("key4".into_key(), Value::DoubleQuotedString("abc".into())),
            ])
        );
        assert!(value_map("(123=bac)").is_err());
        assert!(value_map("(key1=123 2key2=345)").is_err());
    }

    #[test]
    fn test_key_value() {
        let (input, output) = test_parser("key=123.45", key_value);
        assert_eq!(input, "");
        assert_eq!(output, ("key".into_key(), Value::Number(123.45)));
        let (input, output) = test_parser("key=value", key_value);
        assert_eq!(input, "");
        assert_eq!(output, ("key".into_key(), Value::Literal("value".into())));
        let (input, output) = test_parser("key='abc~'def~'\nghi'", key_value);
        assert_eq!(input, "");
        assert_eq!(output, ("key".into_key(), Value::SingleQuotedString("abc~'def~'\nghi".into())));
        let (input, output) = test_parser("key=\"abc'\ndef~\"ghi~\"\"", key_value);
        assert_eq!(input, "");
        assert_eq!(output, ("key".into_key(), Value::DoubleQuotedString("abc'\ndef~\"ghi~\"".into())));
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
        )group(level=1 trailer.height=76 by= (  "col1",   'col2' ))
        compute(band=trailer.5 alignment="2"name=compute_1 expression="count(jw_no for group 5 )+~"件~""x1="0"  )
        "#;
        let dw = test_parser(dwsyn, parse);
        assert_eq!(dw, DWSyntax {
            name: None,
            comment: None,
            version: 12.5,
            datawindow: HashMap::from([
                ("empty_dqt".into_key(), Value::DoubleQuotedString("".into())),
                ("empty_sqt".into_key(), Value::SingleQuotedString("".into())),
                ("num".into_key(), Value::Number(1073741824.)),
                ("lit".into_key(), Value::Literal("yes".into()))
            ]),
            header: HashMap::from([
                ("num".into_key(), Value::Number(564.)),
                ("dqt".into_key(), Value::DoubleQuotedString("536870912".into())),
                ("sqt".into_key(), Value::SingleQuotedString("100".into()))
            ]),
            summary: Default::default(),
            footer: Default::default(),
            detail: Default::default(),
            table: ItemTable {
                columns: vec![ItemTableColumn {
                    name: Some("col1".into_key()),
                    values: HashMap::from([
                        ("type".into_key(), Value::Literal("char(80  )".into())),
                        ("updatewhereclause".into_key(), Value::Literal("yes".into())),
                        ("name".into_key(), Value::Literal("col1".into())),
                        ("dbname".into_key(), Value::DoubleQuotedString("col1".into())),
                    ])
                }],
                values: HashMap::from([
                    ("retrieve".into_key(), Value::DoubleQuotedString("SQL\n        CLAUSE ".into())),
                    (
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
                    ),
                ])
            },
            data: Default::default(),
            items: vec![
                Item {
                    kind: "group".into_key(),
                    name: None,
                    id: None,
                    values: HashMap::from([
                        ("level".into_key(), Value::Number(1.)),
                        ("trailer.height".into_key(), Value::Number(76.)),
                        (
                            "by".into_key(),
                            Value::List(vec![
                                Value::DoubleQuotedString("col1".into()),
                                Value::SingleQuotedString("col2".into()),
                            ])
                        ),
                    ])
                },
                Item {
                    kind: "compute".into_key(),
                    name: Some("compute_1".into_key()),
                    id: None,
                    values: HashMap::from([
                        ("band".into_key(), Value::Literal("trailer.5".into())),
                        ("alignment".into_key(), Value::DoubleQuotedString("2".into())),
                        ("name".into_key(), Value::Literal("compute_1".into())),
                        (
                            "expression".into_key(),
                            Value::DoubleQuotedString(r#"count(jw_no for group 5 )+~"件~""#.into())
                        ),
                        ("x1".into_key(), Value::DoubleQuotedString("0".into()))
                    ])
                }
            ]
        });
        assert_eq!(
            dw.to_string(),
            "release 12.5;\r\ndatawindow(empty_dqt=\"\" empty_sqt='' num=1073741824 lit=yes)\r\nheader(num=564 dqt=\"536870912\" sqt='100')\r\ntable(column=(type=char(80  ) updatewhereclause=yes name=col1 dbname=\"col1\")\r\nretrieve=\"SQL\n        CLAUSE \" arguments=((\"a\", string), (\"b\", string)))\r\ngroup(level=1 trailer.height=76 by=(\"col1\", 'col2'))\r\ncompute(band=trailer.5 alignment=\"2\" name=compute_1 expression=\"count(jw_no for group 5 )+~\"件~\"\" x1=\"0\")\r\n"
        );
    }
}
