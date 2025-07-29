/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::json::key::Key;
use crate::{Element, Property, Value};

use super::{JsonPointerItem, JsonQueryable};
use std::slice::Iter;

/*
impl<T: JsonQueryable> JsonQueryable for Vec<T> {
    fn eval_pointer<'x>(
        &'x self,
        mut pointer: Iter<JsonPointerItem>,
        results: &mut Vec<&'x dyn JsonQueryable>,
    ) {
        match pointer.next() {
            Some(JsonPointerItem::Number(n)) => {
                if let Some(v) = self.get(*n as usize) {
                    v.eval_pointer(pointer, results);
                }
            }
            Some(JsonPointerItem::Wildcard) => {
                for v in self {
                    v.eval_pointer(pointer.clone(), results);
                }
            }
            Some(JsonPointerItem::Root) | None => {
                results.push(self);
            }
            _ => {}
        }
    }
}

impl<V: JsonQueryable, S: BuildHasher + Default> JsonQueryable for HashMap<String, V, S> {
    fn eval_pointer<'x>(
        &'x self,
        mut pointer: Iter<JsonPointerItem>,
        results: &mut Vec<&'x dyn JsonQueryable>,
    ) {
        match pointer.next() {
            Some(JsonPointerItem::String(n)) => {
                if let Some(v) = self.get(n) {
                    v.eval_pointer(pointer, results);
                }
            }
            Some(JsonPointerItem::Number(n)) => {
                let n = n.to_string();
                if let Some(v) = self.get(&n) {
                    v.eval_pointer(pointer, results);
                }
            }
            Some(JsonPointerItem::Wildcard) => {
                for v in self.values() {
                    v.eval_pointer(pointer.clone(), results);
                }
            }
            Some(JsonPointerItem::Root) | None => {
                results.push(self);
            }
        }
    }
}
*/

impl<P: Property + 'static, E: Element + 'static> JsonQueryable for Value<'static, P, E> {
    fn eval_pointer<'x>(
        &'x self,
        mut pointer: Iter<JsonPointerItem>,
        results: &mut Vec<&'x dyn JsonQueryable>,
    ) {
        match pointer.next() {
            Some(JsonPointerItem::String(s)) => {
                if let Value::Object(map) = self {
                    let key = Key::Borrowed(s.as_str());
                    if let Some(v) = map.get(&key) {
                        v.eval_pointer(pointer, results);
                    }
                }
            }
            Some(JsonPointerItem::Number(n)) => match self {
                Value::Array(values) => {
                    if let Some(v) = values.get(*n as usize) {
                        v.eval_pointer(pointer, results);
                    }
                }
                Value::Object(map) => {
                    let n = Key::Owned(n.to_string());
                    if let Some(v) = map.get(&n) {
                        v.eval_pointer(pointer, results);
                    }
                }
                _ => {}
            },
            Some(JsonPointerItem::Wildcard) => match self {
                Value::Array(values) => {
                    for v in values {
                        v.eval_pointer(pointer.clone(), results);
                    }
                }
                Value::Object(map) => {
                    for v in map.values() {
                        v.eval_pointer(pointer.clone(), results);
                    }
                }
                _ => {}
            },
            Some(JsonPointerItem::Root) | None => {
                results.push(self);
            }
        }
    }
}

impl<'x, P: Property, E: Element> Value<'x, P, E> {
    pub fn eval(
        &'x self,
        mut pointer: Iter<JsonPointerItem>,
        results: &mut Vec<&'x Value<'x, P, E>>,
    ) {
        match pointer.next() {
            Some(JsonPointerItem::String(s)) => {
                if let Value::Object(map) = self {
                    let key = Key::Borrowed(s.as_str());
                    if let Some(v) = map.get(&key) {
                        v.eval(pointer, results);
                    }
                }
            }
            Some(JsonPointerItem::Number(n)) => match self {
                Value::Array(values) => {
                    if let Some(v) = values.get(*n as usize) {
                        v.eval(pointer, results);
                    }
                }
                Value::Object(map) => {
                    let n = Key::Owned(n.to_string());
                    if let Some(v) = map.get(&n) {
                        v.eval(pointer, results);
                    }
                }
                _ => {}
            },
            Some(JsonPointerItem::Wildcard) => match self {
                Value::Array(values) => {
                    for v in values {
                        v.eval(pointer.clone(), results);
                    }
                }
                Value::Object(map) => {
                    for v in map.values() {
                        v.eval(pointer.clone(), results);
                    }
                }
                _ => {}
            },
            Some(JsonPointerItem::Root) | None => {
                results.push(self);
            }
        }
    }
}
