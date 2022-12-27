#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
pub use std::borrow::Cow;

#[cfg(feature = "preserve_order")]
pub use indexmap::IndexMap as HashMap;
#[cfg(not(feature = "preserve_order"))]
pub use std::collections::HashMap;

#[cfg(feature = "case_insensitive")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Key<'a>(Cow<'a, str>);

#[cfg(feature = "case_insensitive")]
impl std::fmt::Debug for Key<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { std::fmt::Debug::fmt(&self.0, f) }
}
#[cfg(feature = "case_insensitive")]
impl std::fmt::Display for Key<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { std::fmt::Display::fmt(&self.0, f) }
}
#[cfg(feature = "case_insensitive")]
impl<'a> std::ops::Deref for Key<'a> {
    type Target = Cow<'a, str>;
    #[inline]
    fn deref<'b>(&'b self) -> &'b Self::Target { &self.0 }
}
#[cfg(feature = "case_insensitive")]
impl<'a> std::ops::DerefMut for Key<'a> {
    #[inline]
    fn deref_mut<'b>(&'b mut self) -> &'b mut Self::Target { &mut self.0 }
}
#[cfg(feature = "case_insensitive")]
impl AsRef<str> for Key<'_> {
    #[inline]
    fn as_ref(&self) -> &str { self.0.as_ref() }
}
#[cfg(feature = "case_insensitive")]
impl<T: AsRef<str>> PartialEq<T> for Key<'_> {
    #[inline]
    fn eq(&self, other: &T) -> bool { self.as_ref().eq_ignore_ascii_case(other.as_ref()) }
}
#[cfg(feature = "case_insensitive")]
impl Eq for Key<'_> {}
#[cfg(feature = "case_insensitive")]
impl PartialEq<Key<'_>> for String {
    #[inline]
    fn eq(&self, other: &Key) -> bool { other == self }
}
#[cfg(feature = "case_insensitive")]
impl<'a> PartialEq<Key<'a>> for &'a str {
    #[inline]
    fn eq(&self, other: &Key) -> bool { other == self }
}
#[cfg(feature = "case_insensitive")]
impl std::hash::Hash for Key<'_> {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        for byte in self.as_ref().bytes().map(|b| b.to_ascii_lowercase()) {
            hasher.write_u8(byte);
        }
    }
}

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
    fn into_key(self) -> Key<'a> { Key(self.into()) }
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
