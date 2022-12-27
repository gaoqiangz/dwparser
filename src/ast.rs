use crate::{parser, prelude::*};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// DataWindow语法结构
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DWSyntax<'a> {
    /// `.srd`文件对象名
    ///
    /// # Syntax
    ///
    /// ```txt
    /// $PBExportHeader$name.srd
    /// ```
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub name: Option<Cow<'a, str>>,
    /// `.srd`文件备注
    ///
    /// # Syntax
    ///
    /// ```txt
    /// $PBExportComments$comment
    /// ```
    #[cfg_attr(feature = "serde", serde(borrow))]
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
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub datawindow: HashMap<Key<'a>, Value<'a>>,
    /// `header`语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// header(key=value key=value)
    /// ```
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub header: HashMap<Key<'a>, Value<'a>>,
    /// `summary`语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// summary(key=value key=value)
    /// ```
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub summary: HashMap<Key<'a>, Value<'a>>,
    /// `footer`语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// footer(key=value key=value)
    /// ```
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub footer: HashMap<Key<'a>, Value<'a>>,
    /// `detail`语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// detail(key=value key=value)
    /// ```
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub detail: HashMap<Key<'a>, Value<'a>>,
    /// `table`语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// table(column=(type=type) column=(type=type) key=value key=value)
    /// ```
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub table: ItemTable<'a>,
    /// `data`项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// data(0,1,2)
    /// ```
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub data: Vec<Value<'a>>,
    /// 普通语法项
    ///
    /// # Syntax
    ///
    /// ```txt
    /// text(key=value)
    /// compute(key=value key=value)
    /// ```
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub items: Vec<Item<'a>>
}

impl<'a> DWSyntax<'a> {
    /// 解析语法
    pub fn parse(input: &'a str) -> Result<Self, String> {
        parser::parse(input).map_err(|e| parser::friendly_error(input, e))
    }

    /// 获取指定语法项的参数值
    ///
    /// 兼容`DataWindow::Describe`参数和返回值
    #[cfg(feature = "query")]
    pub fn describe(&self, selector: &str) -> String {
        match parser::query::find(self, selector) {
            Ok(Some(v)) => v.to_string(),
            Ok(None) => {
                #[cfg(feature = "case_insensitive")]
                let selector = selector.to_ascii_lowercase();
                match selector.trim() {
                    "datawindow.column.count" => self.table.columns.len().to_string(),
                    "datawindow.syntax" => self.to_string(),
                    "datawindow.objects" => {
                        let mut objects = String::with_capacity(self.items.len() * 20);
                        for item in &self.items {
                            if let Some(name) = &item.name {
                                if !objects.is_empty() {
                                    objects += "\t";
                                }
                                objects += name;
                            }
                        }
                        objects
                    },
                    _ => "?".to_owned()
                }
            },
            Err(_) => "!".to_owned()
        }
    }

    /// 获取指定语法项的参数值
    ///
    /// 兼容`DataWindow::Describe`参数
    #[cfg(feature = "query")]
    pub fn describe_value<'b>(&'b self, selector: &str) -> Result<Option<&'b Value<'a>>, String> {
        parser::query::find(self, selector).map_err(|e| parser::friendly_error(selector, e))
    }

    /// 修改语法项的参数值
    ///
    /// 兼容`DataWindow::Modify`参数和返回值
    #[cfg(feature = "query")]
    pub fn modify(&mut self, modifier: &str) -> String {
        match parser::query::modify(self, modifier) {
            Ok(_) => "".to_owned(),
            Err(e) => e.to_string()
        }
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
            write!(f, "data({})\r\n", DataDisplay(&self.data))?;
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Item<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub kind: Key<'a>,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub name: Option<Key<'a>>,
    pub id: Option<u32>,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub values: HashMap<Key<'a>, Value<'a>>
}

impl<'a> Item<'a> {
    /// 拷贝值并协变为目标生命期
    pub(crate) fn to_owned<'r>(&self) -> Item<'r> {
        Item {
            kind: Cow::clone(&self.kind).into_owned().into_key(),
            name: self.name.as_ref().map(|v| Cow::clone(v).into_owned().into_key()),
            id: self.id,
            values: self
                .values
                .iter()
                .map(|(k, v)| (Cow::clone(k).into_owned().into_key(), v.to_owned()))
                .collect()
        }
    }
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ItemTable<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub columns: Vec<ItemTableColumn<'a>>,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub values: HashMap<Key<'a>, Value<'a>>
}

impl<'a> ItemTable<'a> {
    pub fn is_empty(&self) -> bool { self.columns.is_empty() && self.values.is_empty() }

    /// 拷贝值并协变为目标生命期
    pub(crate) fn to_owned<'r>(&self) -> ItemTable<'r> {
        ItemTable {
            columns: self.columns.iter().map(|v| v.to_owned()).collect(),
            values: self
                .values
                .iter()
                .map(|(k, v)| (Cow::clone(k).into_owned().into_key(), v.to_owned()))
                .collect()
        }
    }
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ItemTableColumn<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub name: Option<Key<'a>>,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub values: HashMap<Key<'a>, Value<'a>>
}

impl<'a> ItemTableColumn<'a> {
    /// 拷贝值并协变为目标生命期
    pub(crate) fn to_owned<'r>(&self) -> ItemTableColumn<'r> {
        ItemTableColumn {
            name: self.name.as_ref().map(|v| Cow::clone(v).into_owned().into_key()),
            values: self
                .values
                .iter()
                .map(|(k, v)| (Cow::clone(k).into_owned().into_key(), v.to_owned()))
                .collect()
        }
    }
}

impl<'a> Display for ItemTableColumn<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "column=({})", MapDisplay(&self.values))?;
        Ok(())
    }
}

/// 参数值
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Value<'a> {
    /// 字面值
    ///
    /// `literal`
    #[cfg_attr(feature = "serde", serde(rename = "lit"))]
    Literal(Cow<'a, str>),
    /// 双引号字符串
    ///
    /// `"abcd"`
    #[cfg_attr(feature = "serde", serde(rename = "dqt_str"))]
    DoubleQuotedString(Cow<'a, str>),
    /// 单引号字符串
    ///
    /// `'abcd'`
    #[cfg_attr(feature = "serde", serde(rename = "sqt_str"))]
    SingleQuotedString(Cow<'a, str>),
    /// 数值
    ///
    /// `12345`
    #[cfg_attr(feature = "serde", serde(rename = "number"))]
    Number(f64),
    /// Key-Value值列表
    ///
    /// `(key=value key=value)`
    #[cfg_attr(feature = "serde", serde(rename = "map"))]
    Map(HashMap<Key<'a>, Value<'a>>),
    /// 多值列表
    ///
    /// - `("abcd", "abcd")`
    /// - `(("abcd", abcd))`
    #[cfg_attr(feature = "serde", serde(rename = "list"))]
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
    pub fn as_map(&self) -> Option<&HashMap<Key<'a>, Value<'a>>> {
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

    /// 拷贝值并协变为目标生命期
    pub(crate) fn to_owned<'r>(&self) -> Value<'r> {
        match self {
            Value::Literal(v) => Value::Literal(v.clone().into_owned().into()),
            Value::DoubleQuotedString(v) => Value::DoubleQuotedString(v.clone().into_owned().into()),
            Value::SingleQuotedString(v) => Value::SingleQuotedString(v.clone().into_owned().into()),
            Value::Number(v) => Value::Number(*v),
            Value::Map(v) => {
                Value::Map(
                    v.iter().map(|(k, v)| (Cow::clone(k).into_owned().into_key(), v.to_owned())).collect()
                )
            },
            Value::List(v) => Value::List(v.iter().map(|v| v.to_owned()).collect())
        }
    }
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Literal(v) => write!(f, "{v}"),
            Value::DoubleQuotedString(v) => {
                if f.alternate() || v.contains(&['\r', '\n', '\t']) {
                    write!(f, "\"{v}\"")
                } else {
                    if v.contains("~") {
                        write!(f, "{}", v.replace("~\"", "\"").replace("~~", "~"))
                    } else {
                        write!(f, "{v}")
                    }
                }
            },
            Value::SingleQuotedString(v) => {
                if f.alternate() || v.contains(&['\r', '\n', '\t']) {
                    write!(f, "'{v}'")
                } else {
                    if v.contains("~") {
                        write!(f, "{}", v.replace("~'", "'").replace("~~", "~"))
                    } else {
                        write!(f, "{v}")
                    }
                }
            },
            Value::Number(v) => write!(f, "{v}"),
            Value::Map(v) => write!(f, "({})", MapDisplay(&v)),
            Value::List(v) => write!(f, "({})", ListDisplay(&v))
        }
    }
}

struct MapDisplay<'a>(&'a HashMap<Key<'a>, Value<'a>>);

impl<'a> Display for MapDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for (key, value) in self.0 {
            if first {
                write!(f, "{key}={value:#}")?;
            } else {
                write!(f, " {key}={value:#}")?;
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
                write!(f, "{value:#}")?;
            } else {
                write!(f, ", {value:#}")?;
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
            match value {
                Value::Literal(v) => {
                    if v == "null" {
                        write!(f, "{v} ")?;
                    } else {
                        write!(f, "{v}, ")?;
                    }
                },
                _ => {
                    write!(f, "{value:#}, ")?;
                }
            }
        }
        Ok(())
    }
}
