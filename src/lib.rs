mod parser;
mod ast;

mod prelude {
    #[cfg(feature = "indexmap")]
    pub use indexmap::IndexMap as HashMap;
    pub use std::borrow::Cow;
    #[cfg(not(feature = "indexmap"))]
    pub use std::collections::HashMap;
}

pub use ast::*;
pub use prelude::HashMap;
