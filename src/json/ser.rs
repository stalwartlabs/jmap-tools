/*
 * SPDX-FileCopyrightText: 2021 Pascal Seitz <pascal.seitz@gmail.com>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::json::num::{N, Number};
use crate::json::value::Value;
use crate::{Element, Map, Property};
use serde::ser::{Serialize, Serializer};

impl<P: Property, E: Element> Serialize for Value<'_, P, E> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Number(n) => n.serialize(serializer),
            Value::Str(s) => serializer.serialize_str(s),
            Value::Array(v) => serializer.collect_seq(v),
            Value::Object(m) => m.serialize(serializer),
            Value::Element(e) => e.serialize_text(serializer),
        }
    }
}
impl<P: Property, E: Element> Serialize for Map<'_, P, E> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_map(self.iter())
    }
}

impl Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.n {
            N::PosInt(n) => serializer.serialize_u64(n),
            N::NegInt(n) => serializer.serialize_i64(n),
            N::Float(n) => serializer.serialize_f64(n),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::json::de::testkit::{TestElement, TestProp};
    use crate::json::key::Key;
    use crate::json::value::Value;
    use crate::{Element, JsonPointer, JsonPointerItem, Map, Null, Property};
    use serde::Serialize;
    use std::borrow::Cow;

    const TEXTS: &[&str] = &[
        "",
        "name",
        "a\"b\\c",
        "\u{1}\u{1f}\u{7f}",
        "\u{e9}\u{1f389}",
        "a/b~c",
        "\u{2028}",
    ];

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    enum PlainProp {
        Name,
        Text(String),
    }

    impl Property for PlainProp {
        fn try_parse(_: Option<&Key<'_, Self>>, value: &str) -> Option<Self> {
            (value == "name").then_some(PlainProp::Name)
        }

        fn to_cow(&self) -> Cow<'static, str> {
            match self {
                PlainProp::Name => Cow::Borrowed("name"),
                PlainProp::Text(text) => Cow::Owned(text.clone()),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct PlainElement(String);

    impl Element for PlainElement {
        type Property = PlainProp;

        fn try_parse<P>(_: &Key<'_, Self::Property>, value: &str) -> Option<Self> {
            Some(PlainElement(value.to_string()))
        }

        fn to_cow(&self) -> Cow<'static, str> {
            Cow::Owned(self.0.clone())
        }
    }

    fn json<T: Serialize + ?Sized>(value: &T) -> String {
        serde_json::to_string(value).unwrap_or_else(|err| format!("error {err}"))
    }

    fn keys<P: Property>(typed: impl IntoIterator<Item = P>) -> Vec<Key<'static, P>> {
        typed
            .into_iter()
            .map(Key::Property)
            .chain(TEXTS.iter().copied().map(Key::Borrowed))
            .chain(TEXTS.iter().map(|text| Key::Owned(text.to_string())))
            .collect()
    }

    fn check<P: Property, E: Element<Property = P>>(keys: &[Key<'_, P>], elements: &[E]) {
        for key in keys {
            let text = key.to_string();
            let expected = json(text.as_ref());
            assert_eq!(json(key), expected, "{key:?}");
            assert_eq!(
                serde_json::to_value(key).ok(),
                serde_json::to_value(text.as_ref()).ok(),
                "{key:?}"
            );
            let object: Value<'_, P, E> =
                Value::Object(Map::from(vec![(key.clone(), Value::Null)]));
            assert_eq!(json(&object), format!("{{{expected}:null}}"), "{key:?}");
        }
        for element in elements {
            let text = element.to_cow();
            let value: Value<'_, P, E> = Value::Element(element.clone());
            assert_eq!(json(&value), json(text.as_ref()), "{element:?}");
            assert_eq!(
                serde_json::to_value(&value).ok(),
                serde_json::to_value(text.as_ref()).ok(),
                "{element:?}"
            );
        }
    }

    fn pointers() -> Vec<JsonPointer<TestProp>> {
        [
            "a/b",
            "~0/~1",
            "ids/x",
            "/",
            "x/0",
            "x/*",
            "title/a~1b",
            "\u{e9}/\u{1f389}",
        ]
        .iter()
        .map(|text| JsonPointer::parse(text))
        .chain([
            JsonPointer::new(vec![JsonPointerItem::Root]),
            JsonPointer::new(vec![JsonPointerItem::Wildcard, JsonPointerItem::Number(7)]),
            JsonPointer::new(vec![JsonPointerItem::Invalid("x~".into())]),
            JsonPointer::new(vec![JsonPointerItem::Key(Key::Owned("a/b\"".into()))]),
            JsonPointer::new(vec![JsonPointerItem::Key(Key::Property(
                TestProp::Pointer(JsonPointer::parse("c/d")),
            ))]),
        ])
        .collect()
    }

    #[test]
    fn overridden_hooks_serialize_their_text() {
        for pointer in pointers() {
            assert_eq!(json(&pointer), json(&pointer.to_string()), "{pointer:?}");
        }
        let typed = [
            TestProp::Ids,
            TestProp::Title,
            TestProp::Type,
            TestProp::Created,
        ]
        .into_iter()
        .chain(TEXTS.iter().map(|text| TestProp::Id(text.to_string())))
        .chain(pointers().into_iter().map(TestProp::Pointer));
        let elements = TEXTS
            .iter()
            .flat_map(|text| {
                [
                    TestElement::Type(text.to_string()),
                    TestElement::Blob(text.to_string()),
                ]
            })
            .chain([0, 7, u64::MAX].map(TestElement::Created))
            .collect::<Vec<_>>();
        check(&keys(typed), &elements);
    }

    #[test]
    fn default_hooks_serialize_their_text() {
        let typed = [PlainProp::Name]
            .into_iter()
            .chain(TEXTS.iter().map(|text| PlainProp::Text(text.to_string())));
        let elements = TEXTS
            .iter()
            .map(|text| PlainElement(text.to_string()))
            .collect::<Vec<_>>();
        check(&keys(typed), &elements);
        check::<Null, Null>(&keys([Null]), &[Null]);
    }

    #[test]
    fn serialize_json_test() {
        let json_obj =
            r#"{"bool":true,"string_key":"string_val","float":1.23,"i64":-123,"u64":123}"#;

        let val1: Value<'_, Null, Null> = serde_json::from_str(json_obj).unwrap();
        let deser1: String = serde_json::to_string(&val1).unwrap();
        assert_eq!(deser1, json_obj);
    }
}
