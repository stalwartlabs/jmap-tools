/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{JsonPointerHandler, JsonPointerItem};
use crate::json::key::Key;
use crate::pointer::JsonPointerIter;
use crate::{Element, ObjectAsVec, Property, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::BuildHasher;

impl<'x, P: Property, E: Element> JsonPointerHandler<'x, P, E> for Value<'x, P, E> {
    fn eval_jptr<'y>(
        &'y self,
        mut pointer: JsonPointerIter<'_, P>,
        results: &mut Vec<Cow<'y, Value<'x, P, E>>>,
    ) {
        match pointer.next() {
            Some(JsonPointerItem::Key(key)) => {
                if let Value::Object(map) = self
                    && let Some(v) = map.get(key)
                {
                    v.eval_jptr(pointer, results);
                }
            }
            Some(JsonPointerItem::Number(n)) => match self {
                Value::Array(values) => {
                    if let Some(v) = usize::try_from(*n).ok().and_then(|index| values.get(index)) {
                        v.eval_jptr(pointer, results);
                    }
                }
                Value::Object(map) => {
                    let mut buf = NumberKey::default();
                    let n = buf.format(*n);
                    if let Some((_, v)) = map.0.iter().find(|(k, _)| key_matches(k, n)) {
                        v.eval_jptr(pointer, results);
                    }
                }
                _ => {}
            },
            Some(JsonPointerItem::Wildcard) => match self {
                Value::Array(values) => {
                    for v in values {
                        v.eval_jptr(pointer.clone(), results);
                    }
                }
                Value::Object(map) => {
                    for v in map.values() {
                        v.eval_jptr(pointer.clone(), results);
                    }
                }
                _ => {}
            },
            Some(JsonPointerItem::Invalid(_)) => {}
            Some(JsonPointerItem::Root) | None => {
                results.push(Cow::Borrowed(self));
            }
        }
    }

    fn patch_jptr<'y: 'x>(
        &mut self,
        mut pointer: JsonPointerIter<'_, P>,
        value: Value<'y, P, E>,
    ) -> bool {
        match pointer.next() {
            Some(JsonPointerItem::Key(key)) => {
                if let Value::Object(map) = self {
                    return map.patch_entry(|k| k == key, || key.clone(), pointer, value);
                }
            }
            Some(JsonPointerItem::Number(n)) => match self {
                Value::Array(values) => {
                    if let Some(item) = usize::try_from(*n)
                        .ok()
                        .and_then(|index| values.get_mut(index))
                    {
                        return match (pointer.peek().is_some(), value) {
                            (true, value) => item.patch_jptr(pointer, value),
                            (false, Value::Null) => false,
                            (false, value) => {
                                *item = value;
                                true
                            }
                        };
                    }
                }
                Value::Object(map) => {
                    let mut buf = NumberKey::default();
                    let digits = buf.format(*n);
                    return map.patch_entry(
                        |k| key_matches(k, digits),
                        || Key::Owned(digits.to_string()),
                        pointer,
                        value,
                    );
                }
                _ => {}
            },
            Some(
                JsonPointerItem::Wildcard | JsonPointerItem::Root | JsonPointerItem::Invalid(_),
            )
            | None => (),
        }

        false
    }

    fn to_value<'y>(&'y self) -> Cow<'y, Value<'x, P, E>> {
        Cow::Borrowed(self)
    }
}

impl<'x, P: Property, E: Element> ObjectAsVec<'x, P, E> {
    fn patch_entry<'y: 'x>(
        &mut self,
        matches: impl Fn(&Key<'x, P>) -> bool,
        new_key: impl FnOnce() -> Key<'static, P>,
        mut pointer: JsonPointerIter<'_, P>,
        value: Value<'y, P, E>,
    ) -> bool {
        let Some(pos) = self.0.iter().position(|(k, _)| matches(k)) else {
            return match (pointer.peek().is_none(), value) {
                (false, _) => false,
                (true, Value::Null) => true,
                (true, value) => {
                    self.insert_unchecked(new_key(), value);
                    true
                }
            };
        };

        if pointer.peek().is_some() {
            self.0
                .get_mut(pos)
                .is_some_and(|(_, item)| item.patch_jptr(pointer, value))
        } else if matches!(value, Value::Null) {
            self.0.remove(pos);
            true
        } else if let Some((_, item)) = self.0.get_mut(pos) {
            *item = value;
            true
        } else {
            false
        }
    }
}

fn key_matches<P: Property>(key: &Key<'_, P>, digits: &str) -> bool {
    match key {
        Key::Borrowed(key) => *key == digits,
        Key::Owned(key) => key == digits,
        Key::Property(property) => property.to_cow() == digits,
    }
}

#[derive(Default)]
struct NumberKey([u8; 20]);

impl NumberKey {
    fn format(&mut self, mut n: u64) -> &str {
        let mut pos = self.0.len();
        loop {
            pos -= 1;
            if let Some(digit) = self.0.get_mut(pos) {
                *digit = b'0' + (n % 10) as u8;
            }
            n /= 10;
            if n == 0 || pos == 0 {
                break;
            }
        }
        self.0
            .get(pos..)
            .and_then(|digits| std::str::from_utf8(digits).ok())
            .unwrap_or_default()
    }
}

impl<'x, P: Property, E: Element, T> JsonPointerHandler<'x, P, E> for Vec<T>
where
    T: JsonPointerHandler<'x, P, E> + for<'y> TryFrom<Value<'y, P, E>> + 'static,
{
    fn eval_jptr<'y>(
        &'y self,
        mut pointer: JsonPointerIter<'_, P>,
        results: &mut Vec<Cow<'y, Value<'x, P, E>>>,
    ) {
        match pointer.next() {
            Some(JsonPointerItem::Number(n)) => {
                if let Some(v) = usize::try_from(*n).ok().and_then(|index| self.get(index)) {
                    v.eval_jptr(pointer, results);
                }
            }
            Some(JsonPointerItem::Wildcard) => {
                for v in self {
                    v.eval_jptr(pointer.clone(), results);
                }
            }
            Some(JsonPointerItem::Root) | None => {
                results.push(self.to_value());
            }
            _ => {}
        }
    }

    fn patch_jptr<'y: 'x>(
        &mut self,
        mut pointer: JsonPointerIter<'_, P>,
        value: Value<'y, P, E>,
    ) -> bool {
        if let Some(JsonPointerItem::Number(n)) = pointer.next()
            && let Some(item) = usize::try_from(*n)
                .ok()
                .and_then(|index| self.get_mut(index))
        {
            if pointer.peek().is_some() {
                return item.patch_jptr(pointer, value);
            } else if !matches!(value, Value::Null)
                && let Ok(value) = T::try_from(value)
            {
                *item = value;
                return true;
            }
        }
        false
    }

    fn to_value<'y>(&'y self) -> Cow<'y, Value<'x, P, E>> {
        Cow::Owned(Value::Array(
            self.iter().map(|v| v.to_value().into_owned()).collect(),
        ))
    }
}

impl<'x, P: Property, E: Element, T> TryFrom<Value<'x, P, E>> for Vec<T>
where
    T: JsonPointerHandler<'x, P, E> + for<'y> TryFrom<Value<'y, P, E>> + 'static,
{
    type Error = ();

    fn try_from(value: Value<'x, P, E>) -> Result<Self, Self::Error> {
        if let Value::Array(arr) = value {
            arr.into_iter()
                .map(T::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| ())
        } else {
            Err(())
        }
    }
}

impl<'x, P: Property, E: Element, T, S: BuildHasher + Default> JsonPointerHandler<'x, P, E>
    for HashMap<String, T, S>
where
    T: JsonPointerHandler<'x, P, E> + for<'y> TryFrom<Value<'y, P, E>> + 'static,
{
    fn eval_jptr<'y>(
        &'y self,
        mut pointer: JsonPointerIter<'_, P>,
        results: &mut Vec<Cow<'y, Value<'x, P, E>>>,
    ) {
        match pointer.next() {
            Some(JsonPointerItem::Key(key)) => {
                if let Some(v) = self.get(key.to_string().as_ref()) {
                    v.eval_jptr(pointer, results);
                }
            }
            Some(JsonPointerItem::Number(n)) => {
                let mut buf = NumberKey::default();
                if let Some(v) = self.get(buf.format(*n)) {
                    v.eval_jptr(pointer, results);
                }
            }
            Some(JsonPointerItem::Wildcard) => {
                for v in self.values() {
                    v.eval_jptr(pointer.clone(), results);
                }
            }
            Some(JsonPointerItem::Invalid(_)) => {}
            Some(JsonPointerItem::Root) | None => {
                results.push(self.to_value());
            }
        }
    }

    fn patch_jptr<'y: 'x>(
        &mut self,
        mut pointer: JsonPointerIter<'_, P>,
        value: Value<'y, P, E>,
    ) -> bool {
        let mut buf = NumberKey::default();
        let key = match pointer.next() {
            Some(JsonPointerItem::Key(key)) => key.to_string(),
            Some(JsonPointerItem::Number(n)) => Cow::Borrowed(buf.format(*n)),
            Some(
                JsonPointerItem::Wildcard | JsonPointerItem::Root | JsonPointerItem::Invalid(_),
            )
            | None => return false,
        };

        if pointer.peek().is_some() {
            self.get_mut(key.as_ref())
                .is_some_and(|item| item.patch_jptr(pointer, value))
        } else if matches!(value, Value::Null) {
            self.remove(key.as_ref());
            true
        } else if let Ok(value) = T::try_from(value) {
            if let Some(item) = self.get_mut(key.as_ref()) {
                *item = value;
            } else {
                self.insert(key.into_owned(), value);
            }
            true
        } else {
            false
        }
    }

    fn to_value<'y>(&'y self) -> Cow<'y, Value<'x, P, E>> {
        Cow::Owned(Value::Object(
            self.iter()
                .map(|(k, v)| (Key::Owned(k.to_string()), v.to_value().into_owned()))
                .collect(),
        ))
    }
}

impl<'x, P: Property, E: Element, T> TryFrom<Value<'x, P, E>> for HashMap<String, T>
where
    T: JsonPointerHandler<'x, P, E> + for<'y> TryFrom<Value<'y, P, E>> + 'static,
{
    type Error = ();

    fn try_from(value: Value<'x, P, E>) -> Result<Self, Self::Error> {
        if let Value::Object(map) = value {
            map.into_vec()
                .into_iter()
                .map(|(k, v)| T::try_from(v).map(|v| (k.to_string().into_owned(), v)))
                .collect::<Result<HashMap<_, _>, _>>()
                .map_err(|_| ())
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Element, JsonPointer, JsonPointerHandler, JsonPointerItem, Key, Null, ObjectAsVec,
        Property, Value, pointer::JsonPointerIter,
    };
    use serde::{Deserialize, Serialize, Serializer};
    use std::{borrow::Cow, collections::HashMap};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SubObject {
        text: String,
        number: u64,
        boolean: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Object {
        #[serde(serialize_with = "serialize_ordered_map")]
        map: HashMap<String, SubObject>,
        array: Vec<SubObject>,
        value: SubObject,
    }

    #[test]
    fn json_pointer() {
        const TEST: &str = r#"
        {
            "map": {
                "key1": {"text": "value1", "number": 1, "boolean": true},
                "key2": {"text": "value2", "number": 2, "boolean": false}
            },
            "array": [
                {"text": "item1", "number": 10, "boolean": true},
                {"text": "item2", "number": 20, "boolean": false}
            ],
            "value": {"text": "single", "number": 100, "boolean": true}
        }
        "#;

        let obj = serde_json::from_str::<Object>(TEST).unwrap();
        let value = serde_json::from_str::<Value<'static, Null, Null>>(TEST).unwrap();

        test_json_pointer(&obj, "object");
        test_json_pointer(&value, "value");

        test_json_patch(obj, "object");
        test_json_patch(value, "value");
    }

    fn test_json_pointer<T, P, E>(obj: &T, test: &str)
    where
        T: JsonPointerHandler<'static, P, E>,
        P: Property,
        E: Element,
    {
        for (pointer, expected) in [
            ("value/text", r#"["single"]"#),
            ("value/number", r#"[100]"#),
            ("value/boolean", r#"[true]"#),
            (
                "value",
                r#"[{"text":"single","number":100,"boolean":true}]"#,
            ),
            ("map/key1/text", r#"["value1"]"#),
            ("map/key1/number", r#"[1]"#),
            ("map/key1/boolean", r#"[true]"#),
            ("map/key2/text", r#"["value2"]"#),
            ("map/key2/number", r#"[2]"#),
            ("map/key2/boolean", r#"[false]"#),
            ("array/0/text", r#"["item1"]"#),
            ("array/0/number", r#"[10]"#),
            ("array/0/boolean", r#"[true]"#),
            ("array/1/text", r#"["item2"]"#),
            ("array/1/number", r#"[20]"#),
            ("array/1/boolean", r#"[false]"#),
            ("map/*/text", r#"["value1","value2"]"#),
            ("map/*/number", r#"[1,2]"#),
            ("map/*/boolean", r#"[false,true]"#),
            ("map/*/*", r#"["value1","value2",1,2,false,true]"#),
            ("array/*/text", r#"["item1","item2"]"#),
            ("array/*/number", r#"[10,20]"#),
            ("array/*/boolean", r#"[false,true]"#),
            ("array/*/*", r#"["item1","item2",10,20,false,true]"#),
            ("/*/text", r#"["single"]"#),
            ("/*/*/text", r#"["item1","item2","value1","value2"]"#),
            ("/*/*/number", r#"[1,10,2,20]"#),
            ("/*/*/boolean", r#"[false,false,true,true]"#),
        ] {
            let ptr = JsonPointer::parse(pointer);
            let mut results = Vec::new();
            obj.eval_jptr(ptr.iter(), &mut results);
            results.sort_unstable_by_key(|a| a.to_string());
            let results = serde_json::to_string(&results).unwrap();
            if results != expected {
                panic!(
                    "Pointer: {}\nTest: {}\nExpected: {}\nResults: {}",
                    pointer, test, expected, results
                );
            }
        }
    }

    fn test_json_patch<T>(obj: T, test: &str)
    where
        T: JsonPointerHandler<'static, Null, Null> + Clone + Serialize,
    {
        for (pointer, patch, expected) in [
            (
                "value/text",
                r#""hello""#,
                r#""value":{"text":"hello","number":100,"boolean":true}"#,
            ),
            (
                "value/number",
                "123",
                r#""value":{"text":"single","number":123,"boolean":true}"#,
            ),
            (
                "value/boolean",
                "false",
                r#""value":{"text":"single","number":100,"boolean":false}"#,
            ),
            (
                "value",
                r#"{"text":"blah","number":999,"boolean":true}"#,
                r#""value":{"text":"blah","number":999,"boolean":true}"#,
            ),
            (
                "map/key1/text",
                r#""hola""#,
                r#"{"key1":{"text":"hola","number":1,"boolean":true},"key2":{"text":"value2","number":2,"boolean":false}}"#,
            ),
            (
                "map/key1",
                r#"{"text":"adios","number":123,"boolean":false}"#,
                r#"{"key1":{"text":"adios","number":123,"boolean":false},"key2":{"text":"value2","number":2,"boolean":false}}"#,
            ),
            (
                "array/1/text",
                r#""nihao""#,
                r#":[{"text":"item1","number":10,"boolean":true},{"text":"nihao","number":20,"boolean":false}]"#,
            ),
            (
                "array/0",
                r#"{"text":"bonjour","number":42,"boolean":true}"#,
                r#"[{"text":"bonjour","number":42,"boolean":true},{"text":"item2","number":20,"boolean":false}]"#,
            ),
        ] {
            let mut obj = obj.clone();
            obj.patch_jptr(
                JsonPointer::parse(pointer).iter(),
                serde_json::from_str::<Value<'_, Null, Null>>(patch).unwrap(),
            );

            let results = serde_json::to_string(&obj).unwrap();
            if !results.contains(expected) {
                panic!(
                    "Pointer: {}\nTest: {}\nExpected: {}\nResults: {}",
                    pointer, test, expected, results
                );
            }
        }
    }

    fn apply_patch<'a>(document: &'a str, pointer: &str, patch: &'a str) -> (bool, String) {
        let mut value =
            serde_json::from_str::<Value<'a, Null, Null>>(document).expect("valid document");
        let patch = serde_json::from_str::<Value<'a, Null, Null>>(patch).expect("valid patch");
        let applied = value.patch_jptr(JsonPointer::parse(pointer).iter(), patch);
        (
            applied,
            serde_json::to_string(&value).expect("serializable value"),
        )
    }

    #[test]
    fn patch_null_removes_top_level_key() {
        assert_eq!(
            apply_patch(r#"{"a":1,"b":2,"c":3}"#, "b", "null"),
            (true, r#"{"a":1,"c":3}"#.to_string())
        );
    }

    #[test]
    fn patch_null_on_missing_key_is_noop() {
        assert_eq!(
            apply_patch(r#"{"a":1}"#, "b", "null"),
            (true, r#"{"a":1}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"a":{"x":1}}"#, "a/y", "null"),
            (true, r#"{"a":{"x":1}}"#.to_string())
        );
    }

    #[test]
    fn patch_null_removes_nested_key() {
        assert_eq!(
            apply_patch(
                r#"{"overrides":{"r1":{"title":"x"},"r2":{"title":"y"},"r3":{}}}"#,
                "overrides/r2",
                "null"
            ),
            (
                true,
                r#"{"overrides":{"r1":{"title":"x"},"r3":{}}}"#.to_string()
            )
        );
        assert_eq!(
            apply_patch(r#"{"a":{"b":{"c":1,"d":2,"e":3}}}"#, "a/b/d", "null"),
            (true, r#"{"a":{"b":{"c":1,"e":3}}}"#.to_string())
        );
    }

    #[test]
    fn patch_null_through_missing_parent_fails() {
        assert_eq!(
            apply_patch(r#"{"a":1}"#, "b/c", "null"),
            (false, r#"{"a":1}"#.to_string())
        );
    }

    #[test]
    fn patch_null_removes_numeric_object_key() {
        assert_eq!(
            apply_patch(r#"{"m":{"1":"a","2":"b","3":"c"}}"#, "m/2", "null"),
            (true, r#"{"m":{"1":"a","3":"c"}}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"m":{"1":"a"}}"#, "m/2", "null"),
            (true, r#"{"m":{"1":"a"}}"#.to_string())
        );
    }

    #[test]
    fn patch_inside_array() {
        assert_eq!(
            apply_patch(r#"{"l":[1,2,3]}"#, "l/1", "5"),
            (true, r#"{"l":[1,5,3]}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"l":[1,2,3]}"#, "l/1", "null"),
            (false, r#"{"l":[1,2,3]}"#.to_string())
        );
        assert_eq!(
            apply_patch(
                r#"{"name":{"components":[{"value":"Sarah"},{"value":"Connor"}]}}"#,
                "name/components/0",
                "null"
            ),
            (
                false,
                r#"{"name":{"components":[{"value":"Sarah"},{"value":"Connor"}]}}"#.to_string()
            )
        );
        assert_eq!(
            apply_patch(r#"{"l":[1]}"#, "l/4294967296", "2"),
            (false, r#"{"l":[1]}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"l":[{"a":1}]}"#, "l/0/a", "2"),
            (true, r#"{"l":[{"a":2}]}"#.to_string())
        );
        assert_eq!(
            apply_patch(
                r#"{"name":{"components":[{"kind":"given","value":"Sarah"},{"kind":"surname","value":"Connor"}]}}"#,
                "name/components/1/value",
                r#""O'Connor""#
            ),
            (
                true,
                r#"{"name":{"components":[{"kind":"given","value":"Sarah"},{"kind":"surname","value":"O'Connor"}]}}"#
                    .to_string()
            )
        );
        assert_eq!(
            apply_patch(
                r#"{"name":{"components":[{"value":"Sarah"}]}}"#,
                "name/components/0/value",
                "null"
            ),
            (true, r#"{"name":{"components":[{}]}}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"l":[1]}"#, "l/5", "null"),
            (false, r#"{"l":[1]}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"l":[1]}"#, "l/1", "2"),
            (false, r#"{"l":[1]}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"l":[{"a":1}]}"#, "l/3/a", "2"),
            (false, r#"{"l":[{"a":1}]}"#.to_string())
        );
    }

    #[test]
    fn patch_non_null_values_unchanged() {
        assert_eq!(
            apply_patch(r#"{"a":1,"b":2}"#, "a", "5"),
            (true, r#"{"a":5,"b":2}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"a":1}"#, "b", r#""x""#),
            (true, r#"{"a":1,"b":"x"}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"m":{"1":"a"}}"#, "m/1", r#""z""#),
            (true, r#"{"m":{"1":"z"}}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"m":{"1":"a"}}"#, "m/2", r#""z""#),
            (true, r#"{"m":{"1":"a","2":"z"}}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"m":{"1":"a"}}"#, "m/12/x", r#""z""#),
            (false, r#"{"m":{"1":"a"}}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"m":{"12":{"x":1}}}"#, "m/12/x", r#""z""#),
            (true, r#"{"m":{"12":{"x":"z"}}}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"a":{}}"#, "a/b", "false"),
            (true, r#"{"a":{"b":false}}"#.to_string())
        );
    }

    #[test]
    fn patch_null_escaped_slash_key() {
        let override_patch = r#"{"o":{"R":{"participants/abc":null,"title":"t"}}}"#;
        assert_eq!(
            apply_patch(override_patch, "o/R/participants~1abc", "null"),
            (true, r#"{"o":{"R":{"title":"t"}}}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"o":{"R":{}}}"#, "o/R/participants~1abc", "null"),
            (true, r#"{"o":{"R":{}}}"#.to_string())
        );
        assert_eq!(
            apply_patch(override_patch, "o/R/participants/abc", "null"),
            (false, override_patch.to_string())
        );
        assert_eq!(
            apply_patch(
                r#"{"o":{"R":{"participants":{"abc":{},"def":{}}}}}"#,
                "o/R/participants/abc",
                "null"
            ),
            (
                true,
                r#"{"o":{"R":{"participants":{"def":{}}}}}"#.to_string()
            )
        );
    }

    #[test]
    fn patch_null_removes_hashmap_entry() {
        let mut map = HashMap::<String, SubObject>::new();
        map.insert(
            "k".to_string(),
            SubObject {
                text: "t".to_string(),
                number: 1,
                boolean: true,
            },
        );
        assert!(map.patch_jptr(JsonPointer::parse("k").iter(), Value::Null));
        assert!(map.is_empty());
        assert!(map.patch_jptr(JsonPointer::parse("missing").iter(), Value::Null));
        assert!(map.is_empty());
    }

    #[test]
    fn patch_null_removes_nested_hashmap_entry() {
        let sub = SubObject {
            text: "t".to_string(),
            number: 1,
            boolean: true,
        };
        let mut obj = Object {
            map: HashMap::from([
                ("a".to_string(), sub.clone()),
                ("7".to_string(), sub.clone()),
            ]),
            array: vec![sub.clone()],
            value: sub,
        };
        assert!(obj.patch_jptr(JsonPointer::parse("map/a").iter(), Value::Null));
        assert!(obj.patch_jptr(JsonPointer::parse("map/7").iter(), Value::Null));
        assert!(obj.map.is_empty());
        assert!(
            obj.patch_jptr(
                JsonPointer::parse("map/9").iter(),
                serde_json::from_str::<Value<'_, Null, Null>>(
                    r#"{"text":"n","number":9,"boolean":false}"#
                )
                .expect("valid patch")
            )
        );
        assert_eq!(obj.map.get("9").map(|sub| sub.number), Some(9));
        assert!(!obj.patch_jptr(JsonPointer::parse("map/missing/text").iter(), Value::Null));
        assert!(!obj.patch_jptr(JsonPointer::parse("array/0").iter(), Value::Null));
        assert!(!obj.patch_jptr(
            JsonPointer::parse("array/4/text").iter(),
            Value::Str("x".into())
        ));
        assert!(obj.patch_jptr(
            JsonPointer::parse("array/0/text").iter(),
            Value::Str("x".into())
        ));
        assert_eq!(obj.array.len(), 1);
        assert_eq!(obj.array.first().map(|sub| sub.text.as_str()), Some("x"));
    }

    #[test]
    fn patch_escaped_recurrence_override_pointer() {
        let event = r#"{"recurrenceOverrides":{"2025-03-05T09:00:00":{"start":"2025-03-05T10:00:00","participants/dG9tQGZvb2Jhci5xlLmNvbQ/participationStatus":"declined"}}}"#;
        assert_eq!(
            apply_patch(
                event,
                "recurrenceOverrides/2025-03-05T09:00:00/participants~1dG9tQGZvb2Jhci5xlLmNvbQ~1participationStatus",
                "null"
            ),
            (
                true,
                r#"{"recurrenceOverrides":{"2025-03-05T09:00:00":{"start":"2025-03-05T10:00:00"}}}"#
                    .to_string()
            )
        );
        assert_eq!(
            apply_patch(
                event,
                "recurrenceOverrides/2025-03-05T09:00:00/participants~1em9lQGZvb2GFtcGxlLmNvbQ~1participationStatus",
                r#""declined""#
            ),
            (
                true,
                r#"{"recurrenceOverrides":{"2025-03-05T09:00:00":{"start":"2025-03-05T10:00:00","participants/dG9tQGZvb2Jhci5xlLmNvbQ/participationStatus":"declined","participants/em9lQGZvb2GFtcGxlLmNvbQ/participationStatus":"declined"}}}"#
                    .to_string()
            )
        );
    }

    #[test]
    fn patch_numeric_token_after_digit_leading_key() {
        assert_eq!(
            apply_patch(
                r#"{"locations":{"9x":{"links":{}}}}"#,
                "locations/9x/links/5",
                r#"{"href":"a"}"#
            ),
            (
                true,
                r#"{"locations":{"9x":{"links":{"5":{"href":"a"}}}}}"#.to_string()
            )
        );
        assert_eq!(
            apply_patch(
                r#"{"a":{"1k":{"c":[0,1,2,3,4,5,6,7,8,9,10,11]}}}"#,
                "a/1k/c/0",
                "99"
            ),
            (
                true,
                r#"{"a":{"1k":{"c":[99,1,2,3,4,5,6,7,8,9,10,11]}}}"#.to_string()
            )
        );
        assert_eq!(
            apply_patch(
                r#"{"addresses":{"3f2504e0-4f89-11d3":{"components":[{"value":"Main Street"}]}}}"#,
                "addresses/3f2504e0-4f89-11d3/components/0/value",
                r#""High Street""#
            ),
            (
                true,
                r#"{"addresses":{"3f2504e0-4f89-11d3":{"components":[{"value":"High Street"}]}}}"#
                    .to_string()
            )
        );
    }

    #[test]
    fn patch_numeric_key_beyond_u64() {
        assert_eq!(
            apply_patch(
                r#"{"keywords":{}}"#,
                "keywords/99999999999999999999",
                "true"
            ),
            (
                true,
                r#"{"keywords":{"99999999999999999999":true}}"#.to_string()
            )
        );
        assert_eq!(
            apply_patch(
                r#"{"keywords":{"18446744073709551615":true}}"#,
                "keywords/18446744073709551616",
                "null"
            ),
            (
                true,
                r#"{"keywords":{"18446744073709551615":true}}"#.to_string()
            )
        );
        assert_eq!(
            apply_patch(
                r#"{"keywords":{"18446744073709551615":true}}"#,
                "keywords/18446744073709551615",
                "null"
            ),
            (true, r#"{"keywords":{}}"#.to_string())
        );
    }

    #[test]
    fn patch_unterminated_escape_keeps_tilde() {
        let participants = r#"{"participants":{"a":{"name":"x"},"b":{"name":"y"}}}"#;
        assert_eq!(
            apply_patch(participants, "participants/a~", "null"),
            (true, participants.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"keywords":{"k1":true}}"#, "keywords/k2~", "true"),
            (true, r#"{"keywords":{"k1":true,"k2~":true}}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"keywords":{"k1~":true}}"#, "keywords/k1~", "null"),
            (true, r#"{"keywords":{}}"#.to_string())
        );
    }

    #[test]
    fn patch_index_and_member_tokens() {
        assert_eq!(
            apply_patch(r#"{"l":[1,2]}"#, "l/01", "5"),
            (false, r#"{"l":[1,2]}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"l":[1,2]}"#, "l/-", "5"),
            (false, r#"{"l":[1,2]}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"m":{}}"#, "m/01", "5"),
            (true, r#"{"m":{"01":5}}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"m":{}}"#, "m/-", "5"),
            (true, r#"{"m":{"-":5}}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"m":{"1":1}}"#, "m/01", "null"),
            (true, r#"{"m":{"1":1}}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"a":"s"}"#, "a/b", "5"),
            (false, r#"{"a":"s"}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"a":null}"#, "a/b", "null"),
            (false, r#"{"a":null}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"a":{"x~y/z":1}}"#, "a/x~0y~1z", "null"),
            (true, r#"{"a":{}}"#.to_string())
        );
        assert_eq!(
            apply_patch(r#"{"a":1}"#, "", "null"),
            (false, r#"{"a":1}"#.to_string())
        );
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct SevenProperty;

    impl Property for SevenProperty {
        fn try_parse(_: Option<&Key<'_, Self>>, _: &str) -> Option<Self> {
            None
        }

        fn to_cow(&self) -> Cow<'static, str> {
            Cow::Borrowed("7")
        }
    }

    #[test]
    fn numeric_token_matches_property_key() {
        let pointer = JsonPointer::<SevenProperty>::parse("7");
        assert_eq!(pointer.as_slice(), [JsonPointerItem::Number(7)]);

        let mut value: Value<'static, SevenProperty, Null> = Value::Object(ObjectAsVec::from(
            vec![(Key::Property(SevenProperty), Value::Bool(true))],
        ));
        let mut results = Vec::new();
        value.eval_jptr(pointer.iter(), &mut results);
        assert_eq!(
            results.first().map(|result| result.as_ref()),
            Some(&Value::Bool(true))
        );

        assert!(value.patch_jptr(pointer.iter(), Value::Bool(false)));
        assert_eq!(
            value,
            Value::Object(ObjectAsVec::from(vec![(
                Key::Property(SevenProperty),
                Value::Bool(false)
            )]))
        );
        assert!(value.patch_jptr(pointer.iter(), Value::Null));
        assert_eq!(value, Value::Object(ObjectAsVec::new()));
    }

    #[test]
    fn number_key_formatting() {
        for n in [0, 7, 10, 12345, u64::MAX] {
            let mut buf = super::NumberKey::default();
            assert_eq!(buf.format(n), n.to_string());
        }
    }

    impl JsonPointerHandler<'static, Null, Null> for Object {
        fn eval_jptr<'y>(
            &'y self,
            mut pointer: JsonPointerIter<'_, Null>,
            results: &mut Vec<Cow<'y, Value<'static, Null, Null>>>,
        ) {
            match pointer.next() {
                Some(JsonPointerItem::Key(key)) => {
                    let key = key.to_string();
                    if key == "map" {
                        self.map.eval_jptr(pointer, results);
                    } else if key == "array" {
                        self.array.eval_jptr(pointer, results);
                    } else if key == "value" {
                        if pointer.peek().is_none() {
                            results.push(self.value.to_value());
                        } else {
                            self.value.eval_jptr(pointer, results);
                        }
                    }
                }
                Some(JsonPointerItem::Wildcard) => {
                    self.map.eval_jptr(pointer.clone(), results);
                    self.array.eval_jptr(pointer.clone(), results);
                    self.value.eval_jptr(pointer.clone(), results);
                }
                Some(JsonPointerItem::Root) | None => {
                    results.push(self.to_value());
                }
                _ => {}
            }
        }

        fn patch_jptr<'y: 'static>(
            &mut self,
            mut pointer: JsonPointerIter<'_, Null>,
            value: Value<'y, Null, Null>,
        ) -> bool {
            if let Some(JsonPointerItem::Key(key)) = pointer.next() {
                let key = key.to_string();
                if pointer.peek().is_some() {
                    if key == "map" {
                        return self.map.patch_jptr(pointer, value);
                    } else if key == "array" {
                        return self.array.patch_jptr(pointer, value);
                    } else if key == "value" {
                        return self.value.patch_jptr(pointer, value);
                    }
                } else if key == "map" {
                    if let Ok(v) = HashMap::<String, SubObject>::try_from(value) {
                        self.map = v;
                        return true;
                    }
                } else if key == "array" {
                    if let Ok(v) = Vec::<SubObject>::try_from(value) {
                        self.array = v;
                        return true;
                    }
                } else if key == "value"
                    && let Ok(v) = SubObject::try_from(value)
                {
                    self.value = v;
                    return true;
                }
            }

            false
        }

        fn to_value<'y>(&'y self) -> Cow<'y, Value<'static, Null, Null>> {
            Cow::Owned(Value::Object(ObjectAsVec::from(vec![
                (Key::Borrowed("map"), self.map.to_value().into_owned()),
                (Key::Borrowed("array"), self.array.to_value().into_owned()),
                (Key::Borrowed("value"), self.value.to_value().into_owned()),
            ])))
        }
    }

    impl JsonPointerHandler<'static, Null, Null> for SubObject {
        fn eval_jptr<'y>(
            &'y self,
            mut pointer: JsonPointerIter<'_, Null>,
            results: &mut Vec<Cow<'y, Value<'_, Null, Null>>>,
        ) {
            match pointer.next() {
                Some(JsonPointerItem::Key(s)) => match s.to_string().as_ref() {
                    "text" => results.push(Cow::Owned(Value::Str(self.text.clone().into()))),
                    "number" => results.push(Cow::Owned(Value::Number(self.number.into()))),
                    "boolean" => results.push(Cow::Owned(Value::Bool(self.boolean))),
                    _ => {}
                },
                Some(JsonPointerItem::Wildcard) if pointer.peek().is_none() => {
                    results.push(Cow::Owned(Value::Str(self.text.clone().into())));
                    results.push(Cow::Owned(Value::Number(self.number.into())));
                    results.push(Cow::Owned(Value::Bool(self.boolean)));
                }
                _ => {}
            }
        }

        fn patch_jptr<'y: 'static>(
            &mut self,
            mut pointer: JsonPointerIter<'_, Null>,
            value: Value<'y, Null, Null>,
        ) -> bool {
            if let Some(JsonPointerItem::Key(s)) = pointer.next() {
                let has_next = pointer.next().is_some();
                match s.to_string().as_ref() {
                    "text" if !has_next => {
                        if let Some(text) = value.into_string() {
                            self.text = text.into_owned();
                            return true;
                        }
                    }
                    "number" if !has_next => {
                        if let Some(number) = value.as_u64() {
                            self.number = number;
                            return true;
                        }
                    }
                    "boolean" if !has_next => {
                        if let Some(boolean) = value.as_bool() {
                            self.boolean = boolean;
                            return true;
                        }
                    }
                    _ => {}
                }
            }

            false
        }

        fn to_value<'y>(&'y self) -> Cow<'y, Value<'static, Null, Null>> {
            Cow::Owned(Value::Object(ObjectAsVec::from(vec![
                (Key::Borrowed("text"), Value::Str(self.text.clone().into())),
                (Key::Borrowed("number"), Value::Number(self.number.into())),
                (Key::Borrowed("boolean"), Value::Bool(self.boolean)),
            ])))
        }
    }

    impl TryFrom<Value<'_, Null, Null>> for SubObject {
        type Error = ();

        fn try_from(value: Value<'_, Null, Null>) -> Result<Self, Self::Error> {
            if let Value::Object(map) = value {
                let text = map
                    .get(&Key::Borrowed("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .into_owned();
                let number = map
                    .get(&Key::Borrowed("number"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let boolean = map
                    .get(&Key::Borrowed("boolean"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                Ok(SubObject {
                    text,
                    number,
                    boolean,
                })
            } else {
                Err(())
            }
        }
    }

    fn serialize_ordered_map<S>(
        map: &HashMap<String, SubObject>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sorted_pairs: Vec<_> = map.iter().collect();
        sorted_pairs.sort_by_key(|(k, _)| *k);

        use serde::ser::SerializeMap;
        let mut map_serializer = serializer.serialize_map(Some(sorted_pairs.len()))?;
        for (k, v) in sorted_pairs {
            map_serializer.serialize_entry(k, v)?;
        }
        map_serializer.end()
    }
}
