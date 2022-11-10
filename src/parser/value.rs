use super::*;

/// 字面值解析
pub fn literal(input: &str) -> ParseResult<Value> {
    /// 普通字面值
    fn normal(input: &str) -> ParseResult<&str> {
        take_while1(|c: char| c.is_alphanumeric() || c == '.' || c == '_')(input)
    }
    /// 带括号的字面值
    fn with_paren(input: &str) -> ParseResult<&str> {
        recognize(tuple((
            terminated(alpha1, multispace0),
            delimited(tag("("), delimited(multispace0, normal, multispace0), tag(")"))
        )))(input)
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
        separated_list0(tag(","), delimited(multispace0, alt((string, literal, list, fail)), multispace0))
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
