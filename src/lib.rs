mod parser;
mod ast;

mod prelude {
    #[cfg(feature = "preserve_order")]
    pub use indexmap::IndexMap as HashMap;
    pub use std::borrow::Cow;
    #[cfg(not(feature = "preserve_order"))]
    pub use std::collections::HashMap;

    #[cfg(feature = "case_insensitive")]
    pub type KeyType<'a> = unicase::Ascii<Cow<'a, str>>;
    #[cfg(not(feature = "case_insensitive"))]
    pub type KeyType<'a> = Cow<'a, str>;

    pub trait IntoKey<'a> {
        fn into_key(self) -> KeyType<'a>;
    }

    #[cfg(feature = "case_insensitive")]
    impl<'a, T> IntoKey<'a> for T
    where
        T: Into<Cow<'a, str>>
    {
        fn into_key(self) -> KeyType<'a> { KeyType::new(self.into()) }
    }

    #[cfg(not(feature = "case_insensitive"))]
    impl<'a, T> IntoKey<'a> for T
    where
        T: Into<Cow<'a, str>>
    {
        fn into_key(self) -> KeyType<'a> { self.into() }
    }
}

pub use ast::*;
pub use prelude::HashMap;
