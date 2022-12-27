pub use std::borrow::Cow;

#[cfg(feature = "preserve_order")]
pub use indexmap::IndexMap as HashMap;
#[cfg(not(feature = "preserve_order"))]
pub use std::collections::HashMap;

#[cfg(not(feature = "case_insensitive"))]
pub type Key<'a> = Cow<'a, str>;
#[cfg(feature = "case_insensitive")]
pub type Key<'a> = key_ci::Key<'a>;

#[cfg(feature = "case_insensitive")]
mod key_ci {
    #[cfg(feature = "serde")]
    use serde::{Deserialize, Serialize};
    use std::borrow::Cow;

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "serde", serde(transparent))]
    pub struct Key<'a>(Cow<'a, str>);

    impl<'a> From<Cow<'a, str>> for Key<'a> {
        fn from(value: Cow<'a, str>) -> Self { Key(value) }
    }

    impl std::fmt::Debug for Key<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Debug::fmt(&self.0, f)
        }
    }

    impl std::fmt::Display for Key<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Display::fmt(&self.0, f)
        }
    }

    impl<'a> std::ops::Deref for Key<'a> {
        type Target = Cow<'a, str>;
        #[inline]
        fn deref<'b>(&'b self) -> &'b Self::Target { &self.0 }
    }

    impl<'a> std::ops::DerefMut for Key<'a> {
        #[inline]
        fn deref_mut<'b>(&'b mut self) -> &'b mut Self::Target { &mut self.0 }
    }

    impl AsRef<str> for Key<'_> {
        #[inline]
        fn as_ref(&self) -> &str { self.0.as_ref() }
    }

    impl<T: AsRef<str>> PartialEq<T> for Key<'_> {
        #[inline]
        fn eq(&self, other: &T) -> bool { self.as_ref().eq_ignore_ascii_case(other.as_ref()) }
    }

    impl Eq for Key<'_> {}

    impl PartialEq<Key<'_>> for String {
        #[inline]
        fn eq(&self, other: &Key) -> bool { other == self }
    }

    impl<'a> PartialEq<Key<'a>> for &'a str {
        #[inline]
        fn eq(&self, other: &Key) -> bool { other == self }
    }

    impl std::hash::Hash for Key<'_> {
        fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
            for byte in self.as_ref().bytes().map(|b| b.to_ascii_lowercase()) {
                hasher.write_u8(byte);
            }
        }
    }
}

pub trait IntoKey<'a> {
    fn into_key(self) -> Key<'a>;
}

impl<'a, T> IntoKey<'a> for T
where
    T: Into<Cow<'a, str>>
{
    fn into_key(self) -> Key<'a> { Key::from(self.into()) }
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
