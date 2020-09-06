use lazy_static::lazy_static;
use regex::{Captures, Regex};
use serde::{de, ser};
use std::{borrow, cmp, convert, env, fmt, hash};

#[derive(Debug, Clone)]
pub struct SubstitutingString {
    raw: String,
    resolved: String,
}

impl SubstitutingString {
    /// # Errors
    ///
    /// Returns `Err` if `raw` contains a reference to an environment variable that doesn't exist.
    pub fn try_new(raw: String) -> Result<Self, ::std::env::VarError> {
        lazy_static! {
            static ref VARIABLE_RE: Regex =
                Regex::new(r#"\$\{?([A-Z0-9_]+)\}?"#).expect("Invalid regex for VARIABLE_RE");
        }

        for caps in VARIABLE_RE.captures_iter(&raw) {
            env::var(&caps[1])?;
        }
        let resolved = VARIABLE_RE
            .replace_all(&raw, |caps: &Captures<'_>| env::var(&caps[1]).unwrap())
            .into_owned();
        Ok(Self { raw, resolved })
    }
}

impl fmt::Display for SubstitutingString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.resolved)
    }
}

impl PartialEq for SubstitutingString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.resolved.eq(&other.resolved)
    }
}

impl<T> PartialEq<T> for SubstitutingString
where
    String: PartialEq<T>,
{
    #[inline]
    fn eq(&self, other: &T) -> bool {
        self.resolved.eq(&other)
    }
}

impl Eq for SubstitutingString {}

impl Ord for SubstitutingString {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.resolved.cmp(&other.resolved)
    }
}

impl PartialOrd for SubstitutingString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl hash::Hash for SubstitutingString {
    #[inline]
    fn hash<H>(&self, mut state: &mut H)
    where
        H: hash::Hasher,
    {
        self.resolved.hash(&mut state)
    }
}

impl<S: ?Sized> convert::AsRef<S> for SubstitutingString
where
    String: AsRef<S>,
{
    #[inline]
    fn as_ref(&self) -> &S {
        self.resolved.as_ref()
    }
}

impl borrow::Borrow<str> for SubstitutingString {
    #[inline]
    fn borrow(&self) -> &str {
        self.resolved.borrow()
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

impl<'de> de::Visitor<'de> for SubstitutingStringVisitor {
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
