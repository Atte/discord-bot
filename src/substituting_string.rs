#![allow(clippy::non_canonical_partial_ord_impl)] // derivative

use derivative::Derivative;
use derive_more::{AsRef, Deref, Display};
use lazy_regex::{Captures, regex};
use serde::{de, ser};
use std::{env, fmt};

#[derive(Derivative, Debug, Clone, Default, Display, AsRef, Deref)]
#[derivative(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display("{resolved}")]
pub struct SubstitutingString {
    #[derivative(
        PartialEq = "ignore",
        PartialOrd = "ignore",
        Ord = "ignore",
        Hash = "ignore"
    )]
    raw: String,
    #[deref]
    #[as_ref(forward)]
    resolved: String,
}

impl SubstitutingString {
    /// # Errors
    ///
    /// Returns `Err` if `raw` contains a reference to an environment variable that doesn't exist.
    pub fn try_new(raw: String) -> Result<Self, ::std::env::VarError> {
        let pattern = regex!(r"\$\{?([A-Z0-9_]+)\}?");

        for caps in pattern.captures_iter(&raw) {
            env::var(&caps[1])?;
        }
        let resolved = pattern
            .replace_all(&raw, |caps: &Captures<'_>| env::var(&caps[1]).unwrap())
            .into_owned();
        Ok(Self { raw, resolved })
    }
}

impl<T> PartialEq<T> for SubstitutingString
where
    String: PartialEq<T>,
{
    #[inline]
    fn eq(&self, other: &T) -> bool {
        self.resolved.eq(other)
    }
}

impl From<SubstitutingString> for String {
    #[inline]
    fn from(ss: SubstitutingString) -> Self {
        ss.resolved
    }
}

impl ser::Serialize for SubstitutingString {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.raw)
    }
}

struct SubstitutingStringVisitor;

impl de::Visitor<'_> for SubstitutingStringVisitor {
    type Value = SubstitutingString;

    #[inline]
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string optionally containing $ENV variables")
    }

    #[inline]
    fn visit_string<E>(self, s: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        SubstitutingString::try_new(s).map_err(de::Error::custom)
    }

    #[inline]
    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        SubstitutingString::try_new(s.to_owned()).map_err(de::Error::custom)
    }
}

impl<'de> de::Deserialize<'de> for SubstitutingString {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(SubstitutingStringVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::SubstitutingString;
    use std::{
        env::{VarError, set_var},
        sync::Once,
    };

    static INIT: Once = Once::new();

    fn init(raw: impl Into<String>) -> Result<SubstitutingString, VarError> {
        INIT.call_once(|| unsafe {
            set_var("FOO", "bar");
        });
        SubstitutingString::try_new(raw.into())
    }

    #[test]
    fn substitution() -> Result<(), VarError> {
        assert_eq!(init("$FOO")?.to_string(), "bar");
        assert_eq!(init("some $FOO words")?.to_string(), "some bar words");
        assert_eq!(init("curlies ${FOO}")?.to_string(), "curlies bar");
        Ok(())
    }

    #[test]
    fn missing_variable() {
        assert!(init("$BAR").is_err());
    }
}
