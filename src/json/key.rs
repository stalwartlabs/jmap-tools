/*
 * SPDX-FileCopyrightText: 2021 Pascal Seitz <pascal.seitz@gmail.com>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::json::value::Property;
use serde::de::{self, DeserializeSeed, Visitor};
use serde::{Serialize, Serializer};
use std::borrow::Cow;
use std::fmt;

#[derive(Debug, Clone, PartialOrd, Ord, Hash)]
pub enum Key<'x, P: Property> {
    Property(P),
    Borrowed(&'x str),
    Owned(String),
}

pub(crate) struct DeserializationContext<'x, P: Property> {
    pub parent_key: Option<&'x Key<'x, P>>,
}

impl<'de, 'x, P: Property> DeserializeSeed<'de> for DeserializationContext<'x, P> {
    type Value = Key<'de, P>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(KeyVisitor { context: &self })
    }
}

struct KeyVisitor<'x, P: Property> {
    context: &'x DeserializationContext<'x, P>,
}

impl<'de, 'x, P: Property> Visitor<'de> for KeyVisitor<'x, P> {
    type Value = Key<'de, P>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_borrowed_str<ERR>(self, value: &'de str) -> Result<Self::Value, ERR>
    where
        ERR: de::Error,
    {
        match P::try_parse(self.context.parent_key, value) {
            Some(word) => Ok(Key::Property(word)),
            None => Ok(Key::Borrowed(value)),
        }
    }

    fn visit_str<ERR>(self, value: &str) -> Result<Self::Value, ERR>
    where
        ERR: de::Error,
    {
        match P::try_parse(self.context.parent_key, value) {
            Some(word) => Ok(Key::Property(word)),
            None => Ok(Key::Owned(value.to_owned())),
        }
    }

    fn visit_string<ERR>(self, value: String) -> Result<Self::Value, ERR>
    where
        ERR: de::Error,
    {
        match P::try_parse(self.context.parent_key, &value) {
            Some(word) => Ok(Key::Property(word)),
            None => Ok(Key::Owned(value)),
        }
    }
}

impl<P: Property> Serialize for Key<'_, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

impl<P: Property> PartialEq for Key<'_, P> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Key::Property(k1), Key::Property(k2)) => k1 == k2,
            (Key::Property(k1), Key::Borrowed(k2)) => k1.to_cow() == *k2,
            (Key::Property(k1), Key::Owned(k2)) => k1.to_cow() == k2.as_str(),
            (Key::Owned(k1), Key::Owned(k2)) => k1 == k2,
            (Key::Owned(k1), Key::Borrowed(k2)) => k1 == k2,
            (Key::Owned(k1), Key::Property(k2)) => k1.as_str() == k2.to_cow(),
            (Key::Borrowed(k1), Key::Borrowed(k2)) => k1 == k2,
            (Key::Borrowed(k1), Key::Owned(k2)) => k1 == k2,
            (Key::Borrowed(k1), Key::Property(k2)) => *k1 == k2.to_cow(),
        }
    }
}

impl<P: Property> Eq for Key<'_, P> {}

impl<P: Property> PartialEq<&str> for Key<'_, P> {
    fn eq(&self, other: &&str) -> bool {
        self.to_string() == *other
    }
}

impl<'x, P: Property> From<&'x str> for Key<'x, P> {
    fn from(s: &'x str) -> Self {
        match P::try_parse(None, s) {
            Some(word) => Key::Property(word),
            None => Key::Borrowed(s),
        }
    }
}

impl<'x, P: Property> From<Key<'x, P>> for Cow<'x, str> {
    fn from(s: Key<'x, P>) -> Self {
        match s {
            Key::Borrowed(s) => Cow::Borrowed(s),
            Key::Owned(s) => Cow::Owned(s),
            Key::Property(word) => word.to_cow(),
        }
    }
}

impl<'x, P: Property> From<Cow<'x, str>> for Key<'x, P> {
    fn from(s: Cow<'x, str>) -> Self {
        match s {
            Cow::Borrowed(s) => Key::Borrowed(s),
            Cow::Owned(s) => Key::Owned(s),
        }
    }
}

impl<P: Property> Key<'_, P> {
    pub fn to_string(&self) -> Cow<'_, str> {
        match self {
            Key::Borrowed(s) => Cow::Borrowed(s),
            Key::Owned(s) => Cow::Borrowed(s.as_str()),
            Key::Property(word) => word.to_cow(),
        }
    }

    pub fn into_string(self) -> String {
        match self {
            Key::Borrowed(s) => s.to_owned(),
            Key::Owned(s) => s,
            Key::Property(word) => word.to_cow().into_owned(),
        }
    }

    pub fn try_into_property(self) -> Option<P> {
        match self {
            Key::Property(word) => Some(word),
            _ => None,
        }
    }

    pub fn into_owned(self) -> Key<'static, P> {
        match self {
            Key::Borrowed(s) => Key::Owned(s.to_owned()),
            Key::Owned(s) => Key::Owned(s),
            Key::Property(word) => Key::Property(word),
        }
    }

    pub fn to_owned(&self) -> Key<'static, P> {
        match self {
            Key::Borrowed(s) => Key::Owned(s.to_string()),
            Key::Owned(s) => Key::Owned(s.clone()),
            Key::Property(word) => Key::Property(word.clone()),
        }
    }

    pub fn as_property(&self) -> Option<&P> {
        match self {
            Key::Property(word) => Some(word),
            _ => None,
        }
    }

    pub fn as_string_key(&self) -> Option<&str> {
        match self {
            Key::Borrowed(s) => Some(s),
            Key::Owned(s) => Some(s.as_str()),
            Key::Property(_) => None,
        }
    }
}

impl<'x, P: Property> From<P> for Key<'x, P> {
    fn from(word: P) -> Self {
        Key::Property(word)
    }
}
