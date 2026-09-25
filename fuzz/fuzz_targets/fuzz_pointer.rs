/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#![no_main]

use jmap_tools::{
    Element, JsonPointer, JsonPointerHandler, JsonPointerItem, Key, Map, Null, Property, Value,
};
use jmap_tools_fuzz::{Name, Tag, ValueWalk};
use libfuzzer_sys::fuzz_target;
use std::borrow::Cow;

const MAX_POINTER_LINES: usize = 16;
const PATCH_MARK: &str = "fuzz-patch";

fuzz_target!(|data: &[u8]| {
    let mut parts = data.splitn(2, |byte| *byte == 0);
    let document = String::from_utf8_lossy(parts.next().unwrap_or_default());
    let pointers = String::from_utf8_lossy(parts.next().unwrap_or_default());
    let case = PointerCase {
        document: &document,
        pointers: &pointers,
    };
    case.check::<Name, Tag>();
    case.check::<Null, Null>();
});

struct PointerCase<'x> {
    document: &'x str,
    pointers: &'x str,
}

impl PointerCase<'_> {
    fn check<P: Property, E: Element<Property = P>>(&self) {
        let value = Value::<P, E>::parse_json(self.document).ok();
        let derived = value
            .as_ref()
            .map(|value| value.pointer_paths())
            .unwrap_or_default();
        for text in self
            .pointers
            .lines()
            .take(MAX_POINTER_LINES)
            .chain(derived.iter().map(String::as_str))
        {
            let pointer = JsonPointer::<P>::parse(text);
            PointerText(&pointer).check(text);
            if let Some(value) = &value {
                Evaluation {
                    value,
                    pointer: &pointer,
                }
                .check(text);
                if let Value::Object(map) = value {
                    MapModel(map).check(&pointer);
                }
            }
        }
    }
}

struct PointerText<'x, P: Property>(&'x JsonPointer<P>);

impl<P: Property> PointerText<'_, P> {
    fn check(&self, text: &str) {
        let pointer = self.0;
        let shown = pointer.to_string();
        let reparsed = JsonPointer::<P>::parse(&shown);
        let leading_empty_key = matches!(
            pointer.first(),
            Some(JsonPointerItem::Key(key)) if key.to_string().is_empty()
        );
        assert!(
            leading_empty_key || reparsed.to_string() == shown,
            "pointer display is not a fixed point of parse for {text:?}: {shown:?} became {reparsed}"
        );
        let encoded = JsonPointer::<P>::encode(text.split('/'));
        let _ = JsonPointer::<P>::parse(&encoded).to_string();
    }
}

struct Evaluation<'x, 'y, P: Property, E: Element<Property = P>> {
    value: &'x Value<'y, P, E>,
    pointer: &'x JsonPointer<P>,
}

impl<P: Property, E: Element<Property = P>> Evaluation<'_, '_, P, E> {
    fn check(&self, text: &str) {
        let mut results = Vec::new();
        self.value.eval_jptr(self.pointer.iter(), &mut results);

        let mark = Value::Str(Cow::Borrowed(PATCH_MARK));
        let mut patched = self.value.clone();
        let concrete = !self.pointer.is_empty()
            && self
                .pointer
                .iter()
                .all(|item| matches!(item, JsonPointerItem::Key(_) | JsonPointerItem::Number(_)));
        if patched.patch_jptr(self.pointer.iter(), mark.clone()) && concrete {
            let mut found = Vec::new();
            patched.eval_jptr(self.pointer.iter(), &mut found);
            assert!(
                found.iter().any(|item| item.as_ref() == &mark),
                "a successful patch at {text:?} is not visible to eval: {found:?}"
            );
        }
        let mut removed = self.value.clone();
        removed.patch_jptr(self.pointer.iter(), Value::Null);
    }
}

struct MapModel<'x, 'y, P: Property, E: Element<Property = P>>(&'x Map<'y, P, E>);

impl<P: Property, E: Element<Property = P>> MapModel<'_, '_, P, E> {
    fn check(&self, pointer: &JsonPointer<P>) {
        for key in pointer.iter().filter_map(JsonPointerItem::as_key) {
            let text = key.to_string();
            let expected = self
                .0
                .as_vec()
                .iter()
                .position(|(member, _)| member.to_string() == text);
            let scanned = self
                .0
                .iter()
                .find(|(member, _)| member.to_string() == text)
                .map(|(_, item)| item);
            let found = self.0.get(key);
            assert_eq!(
                found, scanned,
                "get disagrees with a scan by text for {key:?}"
            );
            assert_eq!(
                self.0.contains_key(key),
                expected.is_some(),
                "contains_key disagrees with a scan by text for {key:?}"
            );
            let mut map = self.0.clone();
            let removed = map.remove(key);
            let mut model = self.0.as_vec().clone();
            let modelled = expected.map(|position| model.swap_remove(position).1);
            assert_eq!(
                removed, modelled,
                "remove returned a different value for {key:?}"
            );
            assert_eq!(
                map.as_vec(),
                &model,
                "remove left a different object for {key:?}"
            );
            let borrowed = Key::<P>::Borrowed(&text);
            assert_eq!(
                self.0.get(&borrowed),
                found,
                "get by the key text differs from get by the key for {key:?}"
            );
        }
    }
}
