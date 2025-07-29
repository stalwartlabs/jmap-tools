/*
 * SPDX-FileCopyrightText: 2021 Pascal Seitz <pascal.seitz@gmail.com>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt;
use std::ops::Deref;

use crate::json::value::Property;

#[derive(Debug, Clone, PartialOrd, Ord, Hash)]
pub enum Key<'a, P: Property> {
    Property(P),
    Borrowed(&'a str),
    Owned(String),
}

impl<'de: 'a, 'a, P: Property> Deserialize<'de> for Key<'a, P> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(KeyVisitor::<P>(std::marker::PhantomData))
    }
}

struct KeyVisitor<P: Property>(std::marker::PhantomData<P>);

impl<'de, P: Property> Visitor<'de> for KeyVisitor<P> {
    type Value = Key<'de, P>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match P::from_str(value) {
            Ok(word) => Ok(Key::Property(word)),
            Err(_) => Ok(Key::Borrowed(value)),
        }
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match P::from_str(value) {
            Ok(word) => Ok(Key::Property(word)),
            Err(_) => Ok(Key::Owned(value.to_owned())),
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match P::from_str(&value) {
            Ok(word) => Ok(Key::Property(word)),
            Err(_) => Ok(Key::Owned(value)),
        }
    }
}

impl<P: Property> Serialize for Key<'_, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}

impl<P: Property> PartialEq for Key<'_, P> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Key::Borrowed(s1), Key::Borrowed(s2)) => s1 == s2,
            (Key::Owned(s1), Key::Owned(s2)) => s1 == s2,
            (Key::Property(w1), Key::Property(w2)) => w1 == w2,
            _ => self.as_ref() == other.as_ref(),
        }
    }
}

impl<P: Property> Eq for Key<'_, P> {}

impl<P: Property> Deref for Key<'_, P> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<P: Property> PartialEq<&str> for Key<'_, P> {
    fn eq(&self, other: &&str) -> bool {
        self.as_ref() == *other
    }
}

impl<'a, P: Property> From<&'a str> for Key<'a, P> {
    fn from(s: &'a str) -> Self {
        match P::from_str(s) {
            Ok(word) => Key::Property(word),
            Err(_) => Key::Borrowed(s),
        }
    }
}

impl<'a, P: Property> From<Key<'a, P>> for Cow<'a, str> {
    fn from(s: Key<'a, P>) -> Self {
        match s {
            Key::Borrowed(s) => Cow::Borrowed(s),
            Key::Owned(s) => Cow::Owned(s),
            Key::Property(word) => Cow::Owned(word.as_ref().to_string()),
        }
    }
}

impl<P: Property> AsRef<str> for Key<'_, P> {
    fn as_ref(&self) -> &str {
        match self {
            Key::Borrowed(s) => s,
            Key::Owned(s) => s,
            Key::Property(word) => word.as_ref(),
        }
    }
}
