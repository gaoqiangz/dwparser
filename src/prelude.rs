pub use std::borrow::Cow;

#[cfg(feature = "preserve_order")]
pub use indexmap::IndexMap as HashMap;
#[cfg(not(feature = "preserve_order"))]
pub use std::collections::HashMap;

#[cfg(feature = "case_insensitive")]
pub type Key<'a> = unicase::Ascii<Cow<'a, str>>;
#[cfg(not(feature = "case_insensitive"))]
pub type Key<'a> = Cow<'a, str>;

pub trait IntoKey<'a> {
    fn into_key(self) -> Key<'a>;
}

#[cfg(feature = "case_insensitive")]
impl<'a, T> IntoKey<'a> for T
where
    T: Into<Cow<'a, str>>
{
    fn into_key(self) -> Key<'a> { Key::new(self.into()) }
}

#[cfg(not(feature = "case_insensitive"))]
impl<'a, T> IntoKey<'a> for T
where
    T: Into<Cow<'a, str>>
{
    fn into_key(self) -> Key<'a> { self.into() }
}

pub trait CowExt<'a, T: ToOwned + ?Sized + 'a> {
    fn borrowed(&self) -> Option<&'a T>;
}

impl<'a, T> CowExt<'a, T> for Cow<'a, T>
where
    T: ToOwned + ?Sized + 'a
{
    fn borrowed(&self) -> Option<&'a T> {
        match self {
            Cow::Borrowed(v) => Some(v),
            _ => None
        }
    }
}
