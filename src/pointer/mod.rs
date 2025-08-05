/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

pub(crate) mod eval;
pub(crate) mod parser;

use std::{borrow::Cow, fmt::Debug, iter::Peekable, slice::Iter};

use crate::{Element, Key, Property, Value};

pub trait JsonPointerHandler<'x, P: Property, E: Element>: Debug {
    fn eval_jptr<'y>(
        &'y self,
        pointer: JsonPointerIter<'_, P>,
        results: &mut Vec<Cow<'y, Value<'x, P, E>>>,
    );
    fn patch_jptr<'y: 'x>(
        &mut self,
        pointer: JsonPointerIter<'_, P>,
        value: Value<'y, P, E>,
    ) -> bool;
    fn to_value<'y>(&'y self) -> Cow<'y, Value<'x, P, E>>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonPointer<P: Property>(pub(crate) Vec<JsonPointerItem<P>>);

pub type JsonPointerIter<'x, P> = Peekable<Iter<'x, JsonPointerItem<P>>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JsonPointerItem<P: Property> {
    Root,
    Wildcard,
    Key(Key<'static, P>),
    Number(u64),
}

impl<P: Property> JsonPointer<P> {
    pub fn iter(&self) -> JsonPointerIter<'_, P> {
        self.0.iter().peekable()
    }

    pub fn encode<I, T>(items: I) -> String
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let mut encoded = String::with_capacity(8);
        for (pos, item) in items.into_iter().enumerate() {
            if pos > 0 {
                encoded.push('/');
            }
            let item = item.as_ref();
            for c in item.chars() {
                match c {
                    '~' => encoded.push_str("~0"),
                    '/' => encoded.push_str("~1"),
                    _ => encoded.push(c),
                }
            }
        }
        encoded
    }
}

impl<P: Property> JsonPointerItem<P> {
    pub fn as_key(&self) -> Option<&Key<'static, P>> {
        match self {
            JsonPointerItem::Key(key) => Some(key),
            _ => None,
        }
    }
}
