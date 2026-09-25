/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::value::Value;
use crate::json::key::{self, Key};
use crate::json::object_vec::ObjectAsVec;
use crate::{Element, Property};
use serde::de::{Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::borrow::Cow;

type Member<'de, P, E> = (Key<'de, P>, Value<'de, P, E>);

struct Scratch<'de, P: Property, E: Element> {
    members: Vec<Member<'de, P, E>>,
    items: Vec<Value<'de, P, E>>,
}

impl<P: Property, E: Element> Scratch<'_, P, E> {
    fn new() -> Self {
        Scratch {
            members: Vec::new(),
            items: Vec::new(),
        }
    }
}

struct ValueSeed<'s, 'k, 'de, P: Property, E: Element> {
    parent_key: Option<&'k Key<'k, P>>,
    scratch: &'s mut Scratch<'de, P, E>,
}

impl<'de, P: Property, E: Element<Property = P>> Deserialize<'de> for Value<'de, P, E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ValueSeed {
            parent_key: None,
            scratch: &mut Scratch::new(),
        }
        .deserialize(deserializer)
    }
}

impl<'de, P: Property, E: Element<Property = P>> DeserializeSeed<'de>
    for ValueSeed<'_, '_, 'de, P, E>
{
    type Value = Value<'de, P, E>;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de, P: Property, E: Element<Property = P>> ValueSeed<'_, '_, 'de, P, E> {
    #[inline]
    fn string(&self, text: &str) -> Option<E> {
        self.parent_key.and_then(|key| E::try_parse::<P>(key, text))
    }
}

impl<'de, P: Property, E: Element<Property = P>> Visitor<'de> for ValueSeed<'_, '_, 'de, P, E> {
    type Value = Value<'de, P, E>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("any valid JSON value")
    }

    #[inline]
    fn visit_bool<ERR>(self, value: bool) -> Result<Value<'de, P, E>, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Bool(value))
    }

    #[inline]
    fn visit_i64<ERR>(self, value: i64) -> Result<Value<'de, P, E>, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Number(value.into()))
    }

    #[inline]
    fn visit_u64<ERR>(self, value: u64) -> Result<Value<'de, P, E>, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Number(value.into()))
    }

    #[inline]
    fn visit_f64<ERR>(self, value: f64) -> Result<Value<'de, P, E>, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Number(value.into()))
    }

    #[inline]
    fn visit_string<ERR>(self, v: String) -> Result<Self::Value, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(match self.string(&v) {
            Some(element) => Value::Element(element),
            None => Value::Str(Cow::Owned(v)),
        })
    }

    #[inline]
    fn visit_str<ERR>(self, v: &str) -> Result<Self::Value, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(match self.string(v) {
            Some(element) => Value::Element(element),
            None => Value::Str(Cow::Owned(v.to_owned())),
        })
    }

    #[inline]
    fn visit_borrowed_str<ERR>(self, v: &'de str) -> Result<Self::Value, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(match self.string(v) {
            Some(element) => Value::Element(element),
            None => Value::Str(Cow::Borrowed(v)),
        })
    }

    #[inline]
    fn visit_none<ERR>(self) -> Result<Value<'de, P, E>, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Null)
    }

    #[inline]
    fn visit_i8<ERR>(self, v: i8) -> Result<Self::Value, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Number((v as i64).into()))
    }

    #[inline]
    fn visit_i16<ERR>(self, v: i16) -> Result<Self::Value, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Number((v as i64).into()))
    }

    #[inline]
    fn visit_i32<ERR>(self, v: i32) -> Result<Self::Value, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Number((v as i64).into()))
    }

    #[inline]
    fn visit_u8<ERR>(self, v: u8) -> Result<Self::Value, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Number((v as u64).into()))
    }

    #[inline]
    fn visit_u16<ERR>(self, v: u16) -> Result<Self::Value, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Number((v as u64).into()))
    }

    #[inline]
    fn visit_u32<ERR>(self, v: u32) -> Result<Self::Value, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Number((v as u64).into()))
    }

    #[inline]
    fn visit_f32<ERR>(self, v: f32) -> Result<Self::Value, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Number((v as f64).into()))
    }

    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Value<'de, P, E>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        self.deserialize(deserializer)
    }

    #[inline]
    fn visit_unit<ERR>(self) -> Result<Value<'de, P, E>, ERR>
    where
        ERR: serde::de::Error,
    {
        Ok(Value::Null)
    }

    #[inline]
    fn visit_seq<V>(self, mut visitor: V) -> Result<Value<'de, P, E>, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let start = self.scratch.items.len();

        while let Some(elem) = visitor.next_element_seed(ValueSeed {
            parent_key: self.parent_key,
            scratch: &mut *self.scratch,
        })? {
            self.scratch.items.push(elem);
        }

        Ok(Value::Array(self.scratch.items.split_off(start)))
    }

    #[inline]
    fn visit_map<V>(self, mut visitor: V) -> Result<Value<'de, P, E>, V::Error>
    where
        V: MapAccess<'de>,
    {
        let start = self.scratch.members.len();

        while let Some(key) = visitor.next_key_seed(key::DeserializationContext {
            parent_key: self.parent_key,
        })? {
            let value = visitor.next_value_seed(ValueSeed {
                parent_key: Some(&key),
                scratch: &mut *self.scratch,
            })?;

            self.scratch.members.push((key, value));
        }

        Ok(Value::Object(ObjectAsVec(
            self.scratch.members.split_off(start),
        )))
    }
}

#[cfg(test)]
mod tests {

    use std::borrow::Cow;

    use crate::{Null, Value};

    #[test]
    fn deserialize_json_test() {
        let json_obj = r#"
            {
                "bool": true,
                "string_key": "string_val",
                "float": 1.23,
                "i64": -123,
                "u64": 123
            }
       "#;

        let val: Value<'_, Null, Null> = serde_json::from_str(json_obj).unwrap();
        assert_eq!(val.get("bool"), &Value::Bool(true));
        assert_eq!(
            val.get("string_key"),
            &Value::Str(Cow::Borrowed("string_val"))
        );
        assert_eq!(val.get("float"), &Value::Number(1.23.into()));
        assert_eq!(val.get("i64"), &Value::Number((-123i64).into()));
        assert_eq!(val.get("u64"), &Value::Number(123u64.into()));
    }

    #[test]
    fn deserialize_json_allow_escaped_strings_in_values() {
        let json_obj = r#"
            {
                "bool": true,
                "string_key": "string\"_val",
                "u64": 123
            }
       "#;

        let val: Value<'_, Null, Null> = serde_json::from_str(json_obj).unwrap();
        assert_eq!(val.get("bool"), &Value::Bool(true));
        assert_eq!(
            val.get("string_key"),
            &Value::Str(Cow::Borrowed("string\"_val"))
        );
    }
}

#[cfg(test)]
pub(crate) mod testkit {
    use crate::json::key::Key;
    use crate::json::value::Value;
    use crate::{Element, JsonPointer, PointerDepth, Property};
    use serde::Serializer;
    use std::borrow::Cow;
    use std::fmt::Write;

    pub(crate) struct Rng(u64);

    impl Rng {
        pub(crate) fn new(seed: u64) -> Self {
            Rng(seed | 1)
        }

        pub(crate) fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        pub(crate) fn below(&mut self, bound: usize) -> usize {
            if bound == 0 {
                0
            } else {
                (self.next() % bound as u64) as usize
            }
        }

        pub(crate) fn chance(&mut self, one_in: u64) -> bool {
            self.next().is_multiple_of(one_in)
        }

        pub(crate) fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
            &items[self.below(items.len())]
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub(crate) enum TestProp {
        Ids,
        Id(String),
        Title,
        Type,
        Created,
        Pointer(JsonPointer<TestProp>),
    }

    impl Property for TestProp {
        fn try_parse(key: Option<&Key<'_, Self>>, value: &str) -> Option<Self> {
            Self::try_parse_nested(key, value, PointerDepth::default())
        }

        fn try_parse_nested(
            key: Option<&Key<'_, Self>>,
            value: &str,
            depth: PointerDepth,
        ) -> Option<Self> {
            match key {
                Some(Key::Property(TestProp::Ids)) => Some(TestProp::Id(value.to_string())),
                None if value.contains('/') => {
                    JsonPointer::parse_nested(value, depth).map(TestProp::Pointer)
                }
                _ => match value {
                    "ids" => Some(TestProp::Ids),
                    "title" => Some(TestProp::Title),
                    "@type" => Some(TestProp::Type),
                    "created" => Some(TestProp::Created),
                    _ => None,
                },
            }
        }

        fn to_cow(&self) -> Cow<'static, str> {
            match self {
                TestProp::Ids => "ids".into(),
                TestProp::Id(id) => id.clone().into(),
                TestProp::Title => "title".into(),
                TestProp::Type => "@type".into(),
                TestProp::Created => "created".into(),
                TestProp::Pointer(pointer) => pointer.to_string().into(),
            }
        }

        fn serialize_text<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match self {
                TestProp::Id(id) => serializer.serialize_str(id),
                TestProp::Pointer(pointer) => serializer.collect_str(pointer),
                property => serializer.serialize_str(&property.to_cow()),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub(crate) enum TestElement {
        Type(String),
        Created(u64),
        Blob(String),
    }

    impl Element for TestElement {
        type Property = TestProp;

        fn try_parse<P>(key: &Key<'_, Self::Property>, value: &str) -> Option<Self> {
            match key {
                Key::Property(TestProp::Type) => {
                    matches!(value, "Event" | "Task").then(|| TestElement::Type(value.to_string()))
                }
                Key::Property(TestProp::Created) => value.parse().ok().map(TestElement::Created),
                Key::Borrowed(text) if text.ends_with("blob") => {
                    Some(TestElement::Blob(value.to_string()))
                }
                Key::Owned(text) if text.ends_with("blob") => {
                    Some(TestElement::Blob(value.to_string()))
                }
                _ => None,
            }
        }

        fn to_cow(&self) -> Cow<'static, str> {
            match self {
                TestElement::Type(text) | TestElement::Blob(text) => text.clone().into(),
                TestElement::Created(value) => value.to_string().into(),
            }
        }

        fn serialize_text<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match self {
                TestElement::Created(value) => serializer.collect_str(value),
                element => serializer.serialize_str(&element.to_cow()),
            }
        }
    }

    pub(crate) type TestValue<'x> = Value<'x, TestProp, TestElement>;

    pub(crate) fn describe<P: Property, E: Element<Property = P>>(
        value: &Value<'_, P, E>,
    ) -> String {
        let mut out = String::new();
        walk(value, &mut out);
        out
    }

    fn walk<P: Property, E: Element<Property = P>>(value: &Value<'_, P, E>, out: &mut String) {
        match value {
            Value::Str(Cow::Borrowed(text)) => {
                let _ = write!(out, "B{text:?}");
            }
            Value::Str(Cow::Owned(text)) => {
                let _ = write!(out, "O{text:?}");
            }
            Value::Array(items) => {
                let _ = write!(out, "[{}:", items.len());
                for item in items {
                    walk(item, out);
                    out.push(',');
                }
                out.push(']');
            }
            Value::Object(map) => {
                let _ = write!(out, "{{{}:", map.len());
                for (key, item) in map.iter() {
                    match key {
                        Key::Property(property) => {
                            let _ = write!(out, "P{property:?}");
                        }
                        Key::Borrowed(text) => {
                            let _ = write!(out, "B{text:?}");
                        }
                        Key::Owned(text) => {
                            let _ = write!(out, "O{text:?}");
                        }
                    }
                    out.push('=');
                    walk(item, out);
                    out.push(',');
                }
                out.push('}');
            }
            other => {
                let _ = write!(out, "{other:?}");
            }
        }
    }

    const KEYS: &[&str] = &[
        "ids",
        "title",
        "@type",
        "created",
        "a/b",
        "ids/x",
        "~0/~1",
        "/",
        "x/0",
        "x/*",
        "x/-1",
        "name",
        "blob",
        "fileblob",
        "",
        "k1",
        "2025-01-01T00:00:00",
        "a~b",
        "entries",
    ];

    const STRINGS: &[&str] = &[
        "Event",
        "Task",
        "Group",
        "123",
        "0",
        "-5",
        "18446744073709551616",
        "",
        "a/b",
        "x",
        "hello world",
        "\u{e9}t\u{e9}",
        "\u{1f389}",
        "~0~1",
        "#ref",
    ];

    const TOKENS: &[&[u8]] = &[
        b"\"",
        b"{",
        b"}",
        b"[",
        b"]",
        b",",
        b":",
        b"\\",
        b"\\u00e9",
        b"\\ud83c\\udf89",
        b"\\ud800",
        b"\\udc00",
        b"\\ud800\\u0041",
        b"\\u0000",
        b"\\uzz",
        b"null",
        b"true",
        b"fals",
        b"1e999",
        b"-0",
        b"0.5",
        b"1.",
        b"01",
        b"-",
        b"18446744073709551616",
        b"-9223372036854775809",
        b"\t",
        b"\r\n",
        b"\x0c",
        b"\x00",
        b"\x1f",
        b"\x7f",
        b"\xef\xbb\xbf",
        b"\xc3\xa9",
        b"\xff",
        b"\xc3",
        b"\"@type\"",
        b"\"\"",
        b"{}",
        b"[]",
        b"\\n",
        b"\\/",
        b"\\b\\f\\r\\t",
        b"\\x",
        b"\\u00",
        b"\\uD83D\\uDE00",
        b"1.5e-7",
        b"-12.25E+3",
        b"1e",
        b"-e",
        b"0.",
        b"[1,]",
        b"{\"a\"1}",
    ];

    fn whitespace(out: &mut Vec<u8>, rng: &mut Rng) {
        if rng.chance(3) {
            for _ in 0..=rng.below(2) {
                out.push(*rng.pick(b" \n\t\r"));
            }
        }
    }

    fn string(out: &mut Vec<u8>, rng: &mut Rng, text: &str) {
        out.push(b'"');
        let escape_all = rng.chance(8);
        for ch in text.chars() {
            match ch {
                '"' => out.extend_from_slice(b"\\\""),
                '\\' => out.extend_from_slice(b"\\\\"),
                '/' if rng.chance(4) => out.extend_from_slice(b"\\/"),
                ch if escape_all && (ch as u32) < 0x10000 => {
                    out.extend_from_slice(format!("\\u{:04x}", ch as u32).as_bytes())
                }
                ch => {
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
            }
        }
        if rng.chance(5) {
            let len = 100 + rng.below(700);
            let at = rng.below(len + 1);
            let marker: &[u8] = rng.pick(&[
                &b"\\n"[..],
                b"\\\"",
                b"\\u00e9",
                b"\x01",
                b"\x1f",
                b"\t",
                b"",
            ]);
            for index in 0..len {
                if index == at {
                    out.extend_from_slice(marker);
                }
                match rng.below(40) {
                    0 => out.extend_from_slice("\u{e9}".as_bytes()),
                    1 => out.extend_from_slice("\u{1f389}".as_bytes()),
                    2 => out.push(b'/'),
                    _ => out.push(b'A' + rng.below(26) as u8),
                }
            }
        }
        if rng.chance(6) {
            for _ in 0..rng.below(80) {
                match rng.below(14) {
                    0 => out.extend_from_slice(b"\\n"),
                    12 => out.extend_from_slice(b"\\b\\f\\r\\t\\/"),
                    13 => out.extend_from_slice(b"\\u0041\\u00FF\\u20ac"),
                    1 => out.extend_from_slice(b"\\\""),
                    2 => out.extend_from_slice(b"\\ud83c\\udf89"),
                    3 => out.extend_from_slice("\u{20ac}".as_bytes()),
                    _ => out.push(b'a' + rng.below(26) as u8),
                }
            }
        }
        out.push(b'"');
    }

    fn number(out: &mut Vec<u8>, rng: &mut Rng) {
        match rng.below(5) {
            0 => out.extend_from_slice(rng.below(1000).to_string().as_bytes()),
            1 => out.extend_from_slice((rng.next() as i64).to_string().as_bytes()),
            2 => out.extend_from_slice(rng.next().to_string().as_bytes()),
            3 => {
                let value = f64::from_bits(rng.next());
                if value.is_finite() {
                    out.extend_from_slice(format!("{value:e}").as_bytes());
                } else {
                    out.extend_from_slice(b"-0");
                }
            }
            _ => out.extend_from_slice(rng.pick(&[
                &b"0"[..],
                b"-0",
                b"-0.0",
                b"1e308",
                b"1e309",
                b"18446744073709551615",
                b"18446744073709551616",
                b"-9223372036854775808",
                b"-9223372036854775809",
                b"123456789012345678901234567890",
                b"0.1e-400",
            ])),
        }
    }

    pub(crate) fn value(out: &mut Vec<u8>, rng: &mut Rng, depth: usize) {
        whitespace(out, rng);
        match if depth == 0 {
            rng.below(4)
        } else {
            rng.below(7)
        } {
            0 => out.extend_from_slice(rng.pick(&[&b"null"[..], b"true", b"false"])),
            1 => number(out, rng),
            2 | 3 => {
                let text = *rng.pick(STRINGS);
                string(out, rng, text)
            }
            4 => {
                out.push(b'[');
                let len = if depth > 40 { 1 } else { rng.below(6) };
                for index in 0..len {
                    if index > 0 {
                        out.push(b',');
                    }
                    value(out, rng, depth - 1);
                }
                whitespace(out, rng);
                out.push(b']');
            }
            _ => {
                out.push(b'{');
                let len = if depth > 40 { 1 } else { rng.below(8) };
                for index in 0..len {
                    if index > 0 {
                        out.push(b',');
                    }
                    whitespace(out, rng);
                    let key = *rng.pick(KEYS);
                    string(out, rng, key);
                    whitespace(out, rng);
                    out.push(b':');
                    value(out, rng, depth - 1);
                }
                whitespace(out, rng);
                out.push(b'}');
            }
        }
        whitespace(out, rng);
    }

    pub(crate) fn document(rng: &mut Rng) -> Vec<u8> {
        let mut out = Vec::new();
        let depth = if rng.chance(50) {
            100 + rng.below(100)
        } else {
            1 + rng.below(6)
        };
        value(&mut out, rng, depth);
        if rng.chance(3) {
            for _ in 0..=rng.below(2) {
                match rng.below(4) {
                    0 if !out.is_empty() => {
                        let at = rng.below(out.len());
                        let end = (at + 1 + rng.below(20)).min(out.len());
                        out.drain(at..end);
                    }
                    1 => {
                        let at = rng.below(out.len() + 1);
                        out.truncate(at);
                    }
                    2 if !out.is_empty() => {
                        let at = rng.below(out.len());
                        out[at] = rng.next() as u8;
                    }
                    _ => {
                        let at = rng.below(out.len() + 1);
                        let token = *rng.pick(TOKENS);
                        out.splice(at..at, token.iter().copied());
                    }
                }
            }
        }
        out
    }
}
