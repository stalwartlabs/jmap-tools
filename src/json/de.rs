/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::value::Value;
use crate::json::key::Key;
use crate::json::object_vec::ObjectAsVec;
use crate::{Element, Property};
use serde::de::{Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::borrow::Cow;

#[derive(Clone, Default)]
struct DeserializationContext<'x, P: Property, E: Element> {
    parent_key: Option<&'x Key<'x, P>>,
    phantom: std::marker::PhantomData<E>,
}

impl<'de, P: Property, E: Element> Deserialize<'de> for Value<'de, P, E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        DeserializationContext {
            parent_key: None,
            phantom: std::marker::PhantomData,
        }
        .deserialize(deserializer)
    }
}

impl<'de, 'x, P: Property, E: Element> DeserializeSeed<'de> for DeserializationContext<'x, P, E> {
    type Value = Value<'de, P, E>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ContextualVisitor { context: &self })
    }
}

struct ContextualVisitor<'x, P: Property, E: Element> {
    context: &'x DeserializationContext<'x, P, E>,
}

impl<'de, 'x, P: Property, E: Element> Visitor<'de> for ContextualVisitor<'x, P, E> {
    type Value = Value<'de, P, E>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
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
        if let Some(element) = self
            .context
            .parent_key
            .and_then(|key| E::try_parse(key, &v))
        {
            Ok(Value::Element(element))
        } else {
            Ok(Value::Str(Cow::Owned(v)))
        }
    }

    #[inline]
    fn visit_str<ERR>(self, v: &str) -> Result<Self::Value, ERR>
    where
        ERR: serde::de::Error,
    {
        if let Some(element) = self.context.parent_key.and_then(|key| E::try_parse(key, v)) {
            Ok(Value::Element(element))
        } else {
            Ok(Value::Str(Cow::Owned(v.to_owned())))
        }
    }

    #[inline]
    fn visit_borrowed_str<ERR>(self, v: &'de str) -> Result<Self::Value, ERR>
    where
        ERR: serde::de::Error,
    {
        if let Some(element) = self.context.parent_key.and_then(|key| E::try_parse(key, v)) {
            Ok(Value::Element(element))
        } else {
            Ok(Value::Str(Cow::Borrowed(v)))
        }
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
        DeserializationContext {
            parent_key: self.context.parent_key,
            phantom: std::marker::PhantomData,
        }
        .deserialize(deserializer)
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
        let mut vec = Vec::with_capacity(visitor.size_hint().unwrap_or(0));

        while let Some(elem) = visitor.next_element()? {
            vec.push(elem);
        }

        Ok(Value::Array(vec))
    }

    #[inline]
    fn visit_map<V>(self, mut visitor: V) -> Result<Value<'de, P, E>, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut values = Vec::with_capacity(visitor.size_hint().unwrap_or(0));

        while let Some(key) = visitor.next_key()? {
            let value = visitor.next_value_seed(DeserializationContext {
                parent_key: Some(&key),
                phantom: std::marker::PhantomData,
            })?;

            values.push((key, value));
        }

        Ok(Value::Object(ObjectAsVec(values)))
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

        let val: Value<Null, Null> = serde_json::from_str(json_obj).unwrap();
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

        let val: Value<Null, Null> = serde_json::from_str(json_obj).unwrap();
        assert_eq!(val.get("bool"), &Value::Bool(true));
        assert_eq!(
            val.get("string_key"),
            &Value::Str(Cow::Borrowed("string\"_val"))
        );
    }
}
