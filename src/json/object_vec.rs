/*
 * SPDX-FileCopyrightText: 2021 Pascal Seitz <pascal.seitz@gmail.com>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#![allow(clippy::useless_conversion)]
#![allow(clippy::useless_asref)]

use crate::{
    Value,
    json::key::Key,
    json::value::{Element, Property},
};

/// Represents a JSON key/value type.
///
/// For performance reasons we use a Vec instead of a Hashmap.
/// This comes with a tradeoff of slower key accesses as we need to iterate and compare.
///
/// The ObjectAsVec struct is a wrapper around a Vec of (&str, Value) pairs.
/// It provides methods to make it easy to migrate from serde_json::Value::Object or
/// serde_json::Map.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ObjectAsVec<'ctx, P: Property, E: Element>(
    pub(crate) Vec<(Key<'ctx, P>, Value<'ctx, P, E>)>,
);

impl<'ctx, P: Property, E: Element> From<Vec<(&'ctx str, Value<'ctx, P, E>)>>
    for ObjectAsVec<'ctx, P, E>
{
    fn from(vec: Vec<(&'ctx str, Value<'ctx, P, E>)>) -> Self {
        Self::from_iter(vec)
    }
}

impl<'ctx, P: Property, E: Element> FromIterator<(&'ctx str, Value<'ctx, P, E>)>
    for ObjectAsVec<'ctx, P, E>
{
    fn from_iter<T: IntoIterator<Item = (&'ctx str, Value<'ctx, P, E>)>>(iter: T) -> Self {
        Self(iter.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}

impl<'ctx, P: Property, E: Element> ObjectAsVec<'ctx, P, E> {
    /// Access to the underlying Vec.
    ///
    /// # Note
    /// Since KeyStrType can be changed via a feature flag avoid using `as_vec` and use other
    /// methods instead. This could be a problem with feature unification, when one crate uses it
    /// as `&str` and another uses it as `Cow<str>`, both will get `Cow<str>`
    #[inline]
    pub fn as_vec(&self) -> &Vec<(Key<'ctx, P>, Value<'ctx, P, E>)> {
        &self.0
    }

    /// Access to the underlying Vec. Keys are normalized to Cow.
    #[inline]
    pub fn into_vec(self) -> Vec<(Key<'ctx, P>, Value<'ctx, P, E>)> {
        self.0
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// ## Performance
    /// As this is backed by a Vec, this searches linearly through the Vec as may be much more
    /// expensive than a `Hashmap` for larger Objects.
    #[inline]
    pub fn get(&self, key: &Key<'_, P>) -> Option<&Value<'ctx, P, E>> {
        self.0
            .iter()
            .find_map(|(k, v)| if k == key { Some(v) } else { None })
    }

    /// Returns a mutable reference to the value corresponding to the key, if it exists.
    ///
    /// ## Performance
    /// As this is backed by a Vec, this searches linearly through the Vec as may be much more
    /// expensive than a `Hashmap` for larger Objects.
    #[inline]
    pub fn get_mut(&mut self, key: &Key<'ctx, P>) -> Option<&mut Value<'ctx, P, E>> {
        self.0
            .iter_mut()
            .find_map(move |(k, v)| if k == key { Some(v) } else { None })
    }

    /// Returns the key-value pair corresponding to the supplied key.
    ///
    /// ## Performance
    /// As this is backed by a Vec, this searches linearly through the Vec as may be much more
    /// expensive than a `Hashmap` for larger Objects.
    #[inline]
    pub fn get_key_value(&self, key: &Key<'_, P>) -> Option<(&str, &Value<'ctx, P, E>)> {
        self.0.iter().find_map(|(k, v)| {
            if k == key {
                Some((k.as_ref(), v))
            } else {
                None
            }
        })
    }

    /// An iterator visiting all key-value pairs
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&Key<'ctx, P>, &Value<'ctx, P, E>)> {
        self.0.iter().map(|(k, v)| (k, v))
    }

    /// Returns the number of elements in the object
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the object contains no elements
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// An iterator visiting all keys
    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = &Key<'ctx, P>> {
        self.0.iter().map(|(k, _)| k)
    }

    /// An iterator visiting all values
    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &Value<'ctx, P, E>> {
        self.0.iter().map(|(_, v)| v)
    }

    /// Returns true if the object contains a value for the specified key.
    ///
    /// ## Performance
    /// As this is backed by a Vec, this searches linearly through the Vec as may be much more
    /// expensive than a `Hashmap` for larger Objects.
    #[inline]
    pub fn contains_key(&self, key: &Key<'ctx, P>) -> bool {
        self.0.iter().any(|(k, _)| k == key)
    }

    /// Inserts a key-value pair into the object.
    /// If the object did not have this key present, `None` is returned.
    /// If the object did have this key present, the value is updated, and the old value is
    /// returned.
    ///
    /// ## Performance
    /// This operation is linear in the size of the Vec because it potentially requires iterating
    /// through all elements to find a matching key.
    #[inline]
    pub fn insert(
        &mut self,
        key: &'ctx str,
        value: Value<'ctx, P, E>,
    ) -> Option<Value<'ctx, P, E>> {
        for (k, v) in &mut self.0 {
            if *k == key {
                return Some(std::mem::replace(v, value));
            }
        }
        // If the key is not found, push the new key-value pair to the end of the Vec
        self.0.push((key.into(), value));
        None
    }

    /// Inserts a key-value pair into the object if the key does not yet exist, otherwise returns a
    /// mutable reference to the existing value.
    ///
    /// ## Performance
    /// This operation might be linear in the size of the Vec because it requires iterating through
    /// all elements to find a matching key, and might add to the end if not found.
    #[inline]
    pub fn insert_or_get_mut(
        &mut self,
        key: &'ctx str,
        value: Value<'ctx, P, E>,
    ) -> &mut Value<'ctx, P, E> {
        // get position to circumvent lifetime issue
        if let Some(pos) = self.0.iter_mut().position(|(k, _)| *k == key) {
            &mut self.0[pos].1
        } else {
            self.0.push((key.into(), value));
            &mut self.0.last_mut().unwrap().1
        }
    }

    /// Inserts a key-value pair into the object and returns the mutable reference of the inserted
    /// value.
    ///
    /// ## Note
    /// The key must not exist in the object. If the key already exists, the object will contain
    /// multiple keys afterwards.
    ///
    /// ## Performance
    /// This operation is amortized constant time, worst case linear time in the size of the Vec
    /// because it potentially requires a reallocation to grow the Vec.
    #[inline]
    pub fn insert_unchecked_and_get_mut(
        &mut self,
        key: &'ctx str,
        value: Value<'ctx, P, E>,
    ) -> &mut Value<'ctx, P, E> {
        self.0.push((key.into(), value));
        let idx = self.0.len() - 1;
        &mut self.0[idx].1
    }
}

impl<'ctx, P: Property, E: Element> From<ObjectAsVec<'ctx, P, E>>
    for serde_json::Map<String, serde_json::Value>
{
    fn from(val: ObjectAsVec<'ctx, P, E>) -> Self {
        val.iter()
            .map(|(key, val)| (key.to_string(), val.into()))
            .collect()
    }
}
impl<'ctx, P: Property, E: Element> From<&ObjectAsVec<'ctx, P, E>>
    for serde_json::Map<String, serde_json::Value>
{
    fn from(val: &ObjectAsVec<'ctx, P, E>) -> Self {
        val.iter()
            .map(|(key, val)| (key.as_ref().to_owned(), val.into()))
            .collect()
    }
}
