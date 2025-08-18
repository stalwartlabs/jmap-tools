/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use super::{JsonPointerHandler, JsonPointerItem};
use crate::json::key::Key;
use crate::pointer::JsonPointerIter;
use crate::{Element, Property, Value};
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
                    if let Some(v) = values.get(*n as usize) {
                        v.eval_jptr(pointer, results);
                    }
                }
                Value::Object(map) => {
                    let n = Key::Owned(n.to_string());
                    if let Some(v) = map.get(&n) {
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
                    if let Some(pos) = map.0.iter().position(|(k, _)| k == key) {
                        return if pointer.peek().is_some() {
                            map.0[pos].1.patch_jptr(pointer, value)
                        } else {
                            map.0[pos].1 = value;
                            true
                        };
                    } else if pointer.next().is_none() {
                        map.insert_unchecked(key.clone(), value);
                        return true;
                    }
                }
            }
            Some(JsonPointerItem::Number(n)) => match self {
                Value::Array(values) => {
                    if let Some(item) = values.get_mut(*n as usize) {
                        return if pointer.peek().is_some() {
                            item.patch_jptr(pointer, value)
                        } else {
                            *item = value;
                            true
                        };
                    }
                }
                Value::Object(map) => {
                    let n = Key::Owned(n.to_string());
                    if let Some(item) = map.get_mut(&n) {
                        return if pointer.peek().is_some() {
                            item.patch_jptr(pointer, value)
                        } else {
                            *item = value;
                            true
                        };
                    }
                }
                _ => {}
            },
            Some(JsonPointerItem::Wildcard) | Some(JsonPointerItem::Root) | None => (),
        }

        false
    }

    fn to_value<'y>(&'y self) -> Cow<'y, Value<'x, P, E>> {
        Cow::Borrowed(self)
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
                if let Some(v) = self.get(*n as usize) {
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
            && let Some(item) = self.get_mut(*n as usize)
        {
            if pointer.peek().is_some() {
                return item.patch_jptr(pointer, value);
            } else if let Ok(value) = T::try_from(value) {
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
                let n = n.to_string();
                if let Some(v) = self.get(&n) {
                    v.eval_jptr(pointer, results);
                }
            }
            Some(JsonPointerItem::Wildcard) => {
                for v in self.values() {
                    v.eval_jptr(pointer.clone(), results);
                }
            }
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
        match pointer.next() {
            Some(JsonPointerItem::Key(key)) => {
                let key = key.to_string();
                if let Some(item) = self.get_mut(key.as_ref()) {
                    if pointer.peek().is_some() {
                        return item.patch_jptr(pointer, value);
                    } else if let Ok(value) = T::try_from(value) {
                        *item = value;
                        return true;
                    }
                } else if pointer.next().is_none()
                    && let Ok(v) = T::try_from(value)
                {
                    self.insert(key.into_owned(), v);
                    return true;
                }
            }
            Some(JsonPointerItem::Number(n)) => {
                if let Some(v) = self.get_mut(&n.to_string()) {
                    return v.patch_jptr(pointer, value);
                }
            }
            Some(JsonPointerItem::Wildcard) | Some(JsonPointerItem::Root) | None => (),
        }

        false
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
                            self.text = text;
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
