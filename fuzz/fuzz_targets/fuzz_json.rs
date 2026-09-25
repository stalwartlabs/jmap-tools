/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#![no_main]

use jmap_tools::{Element, Key, Null, Property, Value};
use jmap_tools_fuzz::{Name, Tag, ValueWalk};
use libfuzzer_sys::fuzz_target;
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let case = JsonCase {
        raw: data,
        document: &text,
    };
    case.check::<Name, Tag>();
    case.check::<Null, Null>();
});

struct JsonCase<'x> {
    raw: &'x [u8],
    document: &'x str,
}

impl JsonCase<'_> {
    fn check<P: Property, E: Element<Property = P>>(&self) {
        let direct = Value::<P, E>::parse_json(self.document);
        let via_serde =
            serde_json::from_str::<Value<'_, P, E>>(self.document).map_err(|err| err.to_string());
        match (&direct, &via_serde) {
            (Ok(direct), Ok(via_serde)) => {
                assert_eq!(
                    format!("{direct:?}"),
                    format!("{via_serde:?}"),
                    "parse_json and serde_json build different values"
                );
                assert_eq!(
                    direct.layout(),
                    via_serde.layout(),
                    "parse_json and serde_json borrow different strings"
                );
            }
            (Err(direct), Err(via_serde)) => assert_eq!(
                direct, via_serde,
                "parse_json and serde_json report different errors"
            ),
            _ => panic!("parse_json and serde_json disagree: {direct:?} against {via_serde:?}"),
        }

        let from_slice =
            serde_json::from_slice::<Value<'_, P, E>>(self.raw).map_err(|err| err.to_string());
        if std::str::from_utf8(self.raw).is_ok() {
            assert_eq!(
                format!("{from_slice:?}"),
                format!("{via_serde:?}"),
                "serde_json from_slice and from_str disagree on valid UTF-8"
            );
        }

        let Ok(value) = direct else {
            return;
        };
        let json = serde_json::to_string(&value).expect("parsed value serializes");
        let reparsed = Value::<P, E>::parse_json(&json)
            .unwrap_or_else(|err| panic!("serialized value does not parse: {err}: {json}"));
        let pretty = serde_json::to_string_pretty(&value).expect("parsed value serializes pretty");
        let from_pretty = Value::<P, E>::parse_json(&pretty)
            .unwrap_or_else(|err| panic!("pretty JSON does not parse: {err}: {pretty}"));
        if !value.has_float() {
            assert_eq!(
                serde_json::to_string(&reparsed).expect("reparsed value serializes"),
                json,
                "JSON serialization is not a fixed point"
            );
            assert_eq!(
                serde_json::to_string(&from_pretty).expect("value serializes"),
                json,
                "pretty and compact JSON describe different values"
            );
        }
        assert!(
            value.clone().into_owned() == value,
            "into_owned changed the value"
        );
        KeyContract(value.keys()).check();
    }
}

struct KeyContract<P: Property>(Vec<Key<'static, P>>);

impl<P: Property> KeyContract<P> {
    fn check(&self) {
        let hasher = BuildHasherDefault::<DefaultHasher>::default();
        for key in &self.0 {
            let text = key.to_string();
            let borrowed = Key::<P>::Borrowed(&text);
            let key_first = *key == borrowed;
            let text_first = borrowed == *key;
            assert!(
                key_first && text_first && *key == text.as_ref(),
                "key {key:?} does not equal its own text {text:?}"
            );
            assert_eq!(
                hasher.hash_one(key),
                hasher.hash_one(&borrowed),
                "key {key:?} hashes unlike its text {text:?}"
            );
            for other in &self.0 {
                assert_eq!(
                    key == other,
                    text == other.to_string(),
                    "key equality of {key:?} and {other:?} disagrees with their text"
                );
            }
        }
    }
}
