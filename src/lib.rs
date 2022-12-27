mod parser;
mod ast;
mod prelude;

pub use ast::*;
pub use prelude::HashMap;

/// 解析语法
pub fn parse(input: &str) -> Result<DWSyntax, String> { DWSyntax::parse(input) }
