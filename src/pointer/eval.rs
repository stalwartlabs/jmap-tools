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
                if let Value::Object(map) = self {
                    if let Some(v) = map.get(key) {
                        v.eval_jptr(pointer, results);
                    }
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
        if let Some(JsonPointerItem::Number(n)) = pointer.next() {
            if let Some(item) = self.get_mut(*n as usize) {
                if pointer.peek().is_some() {
                    return item.patch_jptr(pointer, value);
                } else if let Ok(value) = T::try_from(value) {
                    *item = value;
                    return true;
                }
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
                } else if pointer.next().is_none() {
                    if let Ok(v) = T::try_from(value) {
                        self.insert(key.into_owned(), v);
                        return true;
                    }
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
