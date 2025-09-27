/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

pub(crate) mod eval;
pub(crate) mod parser;

use std::{
    borrow::Cow,
    fmt::{Debug, Display, Formatter},
    iter::Peekable,
    slice::Iter,
};

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JsonPointer<P: Property>(pub(crate) Vec<JsonPointerItem<P>>);

pub type JsonPointerIter<'x, P> = Peekable<Iter<'x, JsonPointerItem<P>>>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    #[allow(clippy::should_implement_trait)]
    pub fn into_iter(self) -> impl Iterator<Item = JsonPointerItem<P>> {
        self.0.into_iter()
    }

    pub fn into_inner(self) -> Vec<JsonPointerItem<P>> {
        self.0
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

    pub fn first(&self) -> Option<&JsonPointerItem<P>> {
        self.0.first()
    }

    pub fn last(&self) -> Option<&JsonPointerItem<P>> {
        self.0.last()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<P: Property> JsonPointerItem<P> {
    pub fn as_key(&self) -> Option<&Key<'static, P>> {
        match self {
            JsonPointerItem::Key(key) => Some(key),
            _ => None,
        }
    }

    pub fn as_property_key(&self) -> Option<&P> {
        match self {
            JsonPointerItem::Key(Key::Property(key)) => Some(key),
            _ => None,
        }
    }

    pub fn as_string_key(&self) -> Option<&str> {
        match self {
            JsonPointerItem::Key(Key::Borrowed(key)) => Some(key),
            JsonPointerItem::Key(Key::Owned(key)) => Some(key),
            _ => None,
        }
    }
}

impl<P: Property> Display for JsonPointer<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, ptr) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, "/")?;
            }

            match ptr {
                JsonPointerItem::Root => {}
                JsonPointerItem::Wildcard => write!(f, "*")?,
                JsonPointerItem::Key(k) => {
                    for c in k.to_string().chars() {
                        match c {
                            '~' => write!(f, "~0")?,
                            '/' => write!(f, "~1")?,
                            _ => write!(f, "{}", c)?,
                        }
                    }
                }
                JsonPointerItem::Number(n) => write!(f, "{}", n)?,
            }
        }
        Ok(())
    }
}
