use crate::{parser, prelude::*};
use std::fmt::{self, Display};

/// DataWindow语法结构
#[derive(Debug, PartialEq)]
pub struct DWSyntax<'a> {
    /// `.srd`文件对象名
    ///
    /// # Syntax
    ///
    /// ```txt
    /// $PBExportHeader$name.srd
    /// ```
    pub name: Option<Cow<'a, str>>,
    /// `.srd`文件备注
    ///
    /// # Syntax
    ///
    /// ```txt
    /// $PBExportComments$comment
    /// ```
    pub comment: Option<Cow<'a, str>>,
    /// 语法版本
    ///
    /// # Syntax
    ///
    /// ```txt
    /// release 19;
    /// ```
    pub version: f64,
    /// `datawindow`语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// datawindow(key=value key=value)
    /// ```
    pub datawindow: HashMap<Cow<'a, str>, Value<'a>>,
    /// `header`语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// header(key=value key=value)
    /// ```
    pub header: HashMap<Cow<'a, str>, Value<'a>>,
    /// `summary`语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// summary(key=value key=value)
    /// ```
    pub summary: HashMap<Cow<'a, str>, Value<'a>>,
    /// `footer`语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// footer(key=value key=value)
    /// ```
    pub footer: HashMap<Cow<'a, str>, Value<'a>>,
    /// `detail`语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// detail(key=value key=value)
    /// ```
    pub detail: HashMap<Cow<'a, str>, Value<'a>>,
    /// `table`语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// table(column=(type=type) column=(type=type) key=value key=value)
    /// ```
    pub table: ItemTable<'a>,
    /// `data`项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// data(0,1,2)
    /// ```
    pub data: Vec<Value<'a>>,
    /// 普通语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// text(key=value)
    /// compute(key=value key=value)
    /// ```
    pub items: Vec<Item<'a>>
}

impl<'a> DWSyntax<'a> {
    /// 解析语法
    pub fn parse(input: &'a str) -> Result<Self, String> {
        parser::parse(input).map_err(|e| parser::friendly_error(input, e))
    }
}

impl<'a> Display for DWSyntax<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "$PBExportHeader${}\r\n", name)?;
        }
        if let Some(comment) = &self.comment {
            write!(f, "$PBExportComments${}\r\n", comment)?;
        }
        write!(f, "release {};\r\n", self.version)?;
        if !self.datawindow.is_empty() {
            write!(f, "datawindow({})\r\n", MapDisplay(&self.datawindow))?;
        }
        if !self.header.is_empty() {
            write!(f, "header({})\r\n", MapDisplay(&self.header))?;
        }
        if !self.summary.is_empty() {
            write!(f, "summary({})\r\n", MapDisplay(&self.summary))?;
        }
        if !self.footer.is_empty() {
            write!(f, "footer({})\r\n", MapDisplay(&self.footer))?;
        }
        if !self.detail.is_empty() {
            write!(f, "detail({})\r\n", MapDisplay(&self.detail))?;
        }
        if !self.table.is_empty() {
            write!(f, "{}\r\n", self.table)?;
        }
        if !self.data.is_empty() {
            write!(f,"data({})\r\n",DataDisplay(&self.data))?;
        }
        for item in &self.items {
            write!(f, "{item}\r\n")?;
        }
        Ok(())
    }
}

/// 普通语法项
///
/// ```txt
/// item(name=name key=value key=value)
/// ```
#[derive(Debug, PartialEq)]
pub struct Item<'a> {
    pub kind: Cow<'a, str>,
    pub name: Option<Cow<'a, str>>,
    pub values: HashMap<Cow<'a, str>, Value<'a>>
}

impl<'a> Display for Item<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.kind, MapDisplay(&self.values))?;
        Ok(())
    }
}

/// `table`语法项
///
/// ```txt
/// table(column=(type=type) column=(type=type) key=value key=value)
/// ```
#[derive(Debug, PartialEq, Default)]
pub struct ItemTable<'a> {
    pub columns: Vec<ItemTableColumn<'a>>,
    pub values: HashMap<Cow<'a, str>, Value<'a>>
}

impl<'a> ItemTable<'a> {
    pub fn is_empty(&self) -> bool { self.columns.is_empty() && self.values.is_empty() }
}

impl<'a> Display for ItemTable<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "table(")?;
        for column in &self.columns {
            write!(f, "{column}\r\n")?;
        }
        Display::fmt(&MapDisplay(&self.values), f)?;
        write!(f, ")")?;
        Ok(())
    }
}

/// `table`字段语法项
///
/// ```txt
/// column=(name=name key=value)
/// ```
#[derive(Debug, PartialEq)]
pub struct ItemTableColumn<'a> {
    pub name: Option<Cow<'a, str>>,
    pub values: HashMap<Cow<'a, str>, Value<'a>>
}

impl<'a> Display for ItemTableColumn<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "column=({})", MapDisplay(&self.values))?;
        Ok(())
    }
}

/// 参数值
#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    /// 字面值
    ///
    /// `literal`
    Literal(Cow<'a, str>),
    /// 双引号字符串
    ///
    /// `"abcd"`
    DoubleQuotedString(Cow<'a, str>),
    /// 单引号字符串
    ///
    /// `'abcd'`
    SingleQuotedString(Cow<'a, str>),
    /// 数值
    ///
    /// `12345`
    Number(f64),
    /// Key-Value值列表
    ///
    /// `(key=value key=value)`
    Map(HashMap<Cow<'a, str>, Value<'a>>),
    /// 多值列表
    ///
    /// - `("abcd", "abcd")`
    /// - `(("abcd", abcd))`
    List(Vec<Value<'a>>)
}

impl<'a> Value<'a> {
    /// 类型是否为字面值
    pub fn is_literal(&self) -> bool { matches!(self, Value::List(_)) }

    /// 类型是否为字符串
    pub fn is_string(&self) -> bool {
        matches!(self, Value::DoubleQuotedString(_) | Value::SingleQuotedString(_))
    }

    /// 类型是否为数值
    pub fn is_number(&self) -> bool { matches!(self, Value::Number(_)) }

    /// 类型是否为Key-Value值列表
    pub fn is_map(&self) -> bool { matches!(self, Value::Map(_)) }

    /// 类型是否为多值列表
    pub fn is_list(&self) -> bool { matches!(self, Value::List(_)) }

    /// 获取字面值
    pub fn as_literal(&self) -> Option<&Cow<'a, str>> {
        match self {
            Value::Literal(v) => Some(v),
            _ => None
        }
    }

    /// 获取字符串
    pub fn as_string(&self) -> Option<&Cow<'a, str>> {
        match self {
            Value::DoubleQuotedString(v) | Value::SingleQuotedString(v) => Some(v),
            _ => None
        }
    }

    /// 获取数值
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(v) => Some(*v),
            _ => None
        }
    }

    /// 获取Key-Value值列表
    pub fn as_map(&self) -> Option<&HashMap<Cow<'a, str>, Value<'a>>> {
        match self {
            Value::Map(v) => Some(v),
            _ => None
        }
    }

    /// 获取多值列表
    pub fn as_list(&self) -> Option<&Vec<Value<'a>>> {
        match self {
            Value::List(v) => Some(v),
            _ => None
        }
    }
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Literal(v) => write!(f, "{v}"),
            Value::DoubleQuotedString(v) => write!(f, "\"{v}\""),
            Value::SingleQuotedString(v) => write!(f, "'{v}'"),
            Value::Number(v) => write!(f, "{v}"),
            Value::Map(v) => write!(f, "({})", MapDisplay(&v)),
            Value::List(v) => write!(f, "({})", ListDisplay(&v))
        }
    }
}

struct MapDisplay<'a>(&'a HashMap<Cow<'a, str>, Value<'a>>);

impl<'a> Display for MapDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for (key, value) in self.0 {
            if first {
                write!(f, "{key}={value}")?;
            } else {
                write!(f, " {key}={value}")?;
            }
            first = false;
        }
        Ok(())
    }
}

struct ListDisplay<'a>(&'a Vec<Value<'a>>);

impl<'a> Display for ListDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for value in self.0 {
            if first {
                write!(f, "{value}")?;
            } else {
                write!(f, ", {value}")?;
            }
            first = false;
        }
        Ok(())
    }
}

struct DataDisplay<'a>(&'a Vec<Value<'a>>);

impl<'a> Display for DataDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for value in self.0 {
            write!(f," ")?;
            if let Value::List(item) = value{
                for v in item{
                    write!(f,"{},",v)?;
                }
            }
        }
        Ok(())
    }
}
