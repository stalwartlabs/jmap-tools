/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use jmap_tools::{Element, JsonPointer, Key, Property, Value};
use std::{borrow::Cow, fmt::Write};

const MAX_KEYS: usize = 64;
const MAX_PATHS: usize = 24;
const MAX_DEPTH: usize = 6;

pub trait ValueWalk<P: Property> {
    fn layout(&self) -> String;
    fn has_float(&self) -> bool;
    fn keys(&self) -> Vec<Key<'static, P>>;
    fn pointer_paths(&self) -> Vec<String>;
}

trait Walker<P: Property> {
    fn walk_layout(&self, out: &mut String);
    fn walk_keys(&self, out: &mut Vec<Key<'static, P>>);
    fn walk_paths(&self, prefix: &mut Vec<String>, out: &mut Vec<String>);
}

impl<P: Property, E: Element<Property = P>> ValueWalk<P> for Value<'_, P, E> {
    fn layout(&self) -> String {
        let mut layout = String::new();
        self.walk_layout(&mut layout);
        layout
    }

    fn has_float(&self) -> bool {
        match self {
            Value::Number(_) => self.is_f64(),
            Value::Array(items) => items.iter().any(|item| item.has_float()),
            Value::Object(map) => map.values().any(|item| item.has_float()),
            _ => false,
        }
    }

    fn keys(&self) -> Vec<Key<'static, P>> {
        let mut keys = Vec::new();
        self.walk_keys(&mut keys);
        keys
    }

    fn pointer_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        self.walk_paths(&mut Vec::new(), &mut paths);
        paths
    }
}

impl<P: Property, E: Element<Property = P>> Walker<P> for Value<'_, P, E> {
    fn walk_layout(&self, out: &mut String) {
        match self {
            Value::Str(Cow::Borrowed(_)) => out.push('b'),
            Value::Str(Cow::Owned(_)) => out.push('o'),
            Value::Array(items) => {
                out.push('[');
                items.iter().for_each(|item| item.walk_layout(out));
                out.push(']');
            }
            Value::Object(map) => {
                out.push('{');
                for (key, item) in map.iter() {
                    let _ = write!(out, "{key:?}");
                    item.walk_layout(out);
                }
                out.push('}');
            }
            _ => out.push('.'),
        }
    }

    fn walk_keys(&self, out: &mut Vec<Key<'static, P>>) {
        match self {
            Value::Array(items) => items.iter().for_each(|item| item.walk_keys(out)),
            Value::Object(map) => {
                for (key, item) in map.iter() {
                    if out.len() >= MAX_KEYS {
                        return;
                    }
                    out.push(key.to_owned());
                    item.walk_keys(out);
                }
            }
            _ => {}
        }
    }

    fn walk_paths(&self, prefix: &mut Vec<String>, out: &mut Vec<String>) {
        if out.len() >= MAX_PATHS || prefix.len() >= MAX_DEPTH {
            return;
        }
        let null = Value::Null;
        let children: Vec<(String, &Self)> = match self {
            Value::Object(map) => map
                .iter()
                .map(|(key, item)| (key.to_string().into_owned(), item))
                .collect(),
            Value::Array(items) => items
                .iter()
                .chain([&null])
                .enumerate()
                .map(|(index, item)| (index.to_string(), item))
                .collect(),
            _ => return,
        };
        for (segment, item) in children {
            prefix.push(segment);
            if out.len() < MAX_PATHS {
                let pointer = JsonPointer::<P>::encode(prefix.iter());
                out.push(format!("/{pointer}"));
                if let Some((_, parent)) = prefix.split_last() {
                    let mut wildcard = JsonPointer::<P>::encode(parent);
                    wildcard.push_str(if parent.is_empty() { "*" } else { "/*" });
                    out.push(wildcard);
                }
                out.push(pointer);
            }
            item.walk_paths(prefix, out);
            prefix.pop();
        }
    }
}
