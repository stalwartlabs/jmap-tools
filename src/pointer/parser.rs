/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::{JsonPointer, JsonPointerItem, Key, PointerDepth, Property};
use std::borrow::Cow;

#[derive(Clone, Copy)]
struct Token<'x> {
    text: &'x str,
    digits: bool,
    escaped: bool,
}

impl<P: Property> JsonPointer<P> {
    pub fn parse(value: &str) -> Self {
        Self::parse_tokens(value, PointerDepth::default().inner())
    }

    pub fn parse_nested(value: &str, depth: PointerDepth) -> Option<Self> {
        depth
            .can_nest()
            .then(|| Self::parse_tokens(value, depth.inner()))
    }

    fn parse_tokens(value: &str, depth: PointerDepth) -> Self {
        let mut path = Vec::with_capacity(value.bytes().filter(|&byte| byte == b'/').count() + 1);
        let (token, mut rest) = Token::split(value);

        if !token.text.is_empty() {
            path.push(JsonPointerItem::parse_token(None, token, depth));
        }

        while let Some(text) = rest {
            let (token, tail) = Token::split(text);
            let item = if token.text.is_empty() {
                JsonPointerItem::Key("".into())
            } else {
                JsonPointerItem::parse_token(
                    path.last().and_then(JsonPointerItem::as_key),
                    token,
                    depth,
                )
            };
            path.push(item);
            rest = tail;
        }

        if path.is_empty() {
            path.push(JsonPointerItem::Root);
        }

        JsonPointer(path)
    }
}

impl<'x> Token<'x> {
    fn split(text: &'x str) -> (Self, Option<&'x str>) {
        let mut digits = true;
        let mut escaped = false;
        let mut end = text.len();
        for (pos, byte) in text.bytes().enumerate() {
            match byte {
                b'/' => {
                    end = pos;
                    break;
                }
                b'~' => {
                    escaped = true;
                    digits = false;
                }
                b'0'..=b'9' => {}
                _ => digits = false,
            }
        }
        let token = Token {
            text: text.get(..end).unwrap_or_default(),
            digits,
            escaped,
        };
        (token, text.get(end + 1..))
    }

    fn unescape(self) -> Option<String> {
        let (head, mut rest) = self.text.split_once('~')?;
        let mut text = String::with_capacity(self.text.len());
        text.push_str(head);
        loop {
            match rest.split_at_checked(1) {
                Some(("0", tail)) => {
                    text.push('~');
                    rest = tail;
                }
                Some(("1", tail)) => {
                    text.push('/');
                    rest = tail;
                }
                _ if rest.is_empty() => {
                    text.push('~');
                    return Some(text);
                }
                _ => return None,
            }

            match rest.split_once('~') {
                Some((segment, tail)) => {
                    text.push_str(segment);
                    rest = tail;
                }
                None => {
                    text.push_str(rest);
                    return Some(text);
                }
            }
        }
    }
}

impl<P: Property> JsonPointerItem<P> {
    fn parse_token(
        parent: Option<&Key<'static, P>>,
        token: Token<'_>,
        depth: PointerDepth,
    ) -> Self {
        let text = token.text;
        if token.digits {
            match P::try_parse_nested(parent, text, depth) {
                Some(prop) => JsonPointerItem::Key(Key::Property(prop)),
                None => match text.parse::<u64>() {
                    Ok(index) if text == "0" || !text.starts_with('0') => {
                        JsonPointerItem::Number(index)
                    }
                    _ => JsonPointerItem::Key(Key::Owned(text.to_string())),
                },
            }
        } else if text == "*" {
            JsonPointerItem::Wildcard
        } else if !token.escaped {
            JsonPointerItem::Key(match P::try_parse_nested(parent, text, depth) {
                Some(prop) => Key::Property(prop),
                None => Key::Owned(text.to_string()),
            })
        } else {
            match token.unescape() {
                Some(text) => {
                    JsonPointerItem::Key(match P::try_parse_nested(parent, &text, depth) {
                        Some(prop) => Key::Property(prop),
                        None => Key::Owned(text),
                    })
                }
                None => JsonPointerItem::Invalid(text.to_string()),
            }
        }
    }
}

impl<'de, P: Property> serde::Deserialize<'de> for JsonPointer<P> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <Cow<'de, str>>::deserialize(deserializer).map(|s| JsonPointer::parse(s.as_ref()))
    }
}

impl<P: Property> serde::Serialize for JsonPointer<P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {

    use super::{JsonPointer, JsonPointerItem};
    use crate::json::de::testkit::{TestProp as Nested, TestValue};
    use crate::{Key, Null, PointerDepth, Property, Value};
    use std::{borrow::Cow, iter::successors, thread};

    const SMALL_STACK: usize = 256 * 1024;
    const DEEP_LEVELS: usize = 200;

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    enum TestProp {
        Ids,
        Id(String),
    }

    impl Property for TestProp {
        fn try_parse(key: Option<&Key<'_, Self>>, value: &str) -> Option<Self> {
            if let Some(Key::Property(TestProp::Ids)) = key {
                Some(TestProp::Id(value.to_string()))
            } else if value == "ids" {
                Some(TestProp::Ids)
            } else {
                None
            }
        }

        fn to_cow(&self) -> Cow<'static, str> {
            match self {
                TestProp::Ids => Cow::Borrowed("ids"),
                TestProp::Id(s) => Cow::Owned(s.clone()),
            }
        }
    }

    #[test]
    fn json_pointer_parse() {
        for (input, output) in vec![
            ("hello", vec![JsonPointerItem::<Null>::Key("hello".into())]),
            ("9a", vec![JsonPointerItem::Key("9a".into())]),
            ("a9", vec![JsonPointerItem::Key("a9".into())]),
            ("*a", vec![JsonPointerItem::Key("*a".into())]),
            (
                "/hello/world",
                vec![
                    JsonPointerItem::Key("hello".into()),
                    JsonPointerItem::Key("world".into()),
                ],
            ),
            ("*", vec![JsonPointerItem::Wildcard]),
            (
                "/hello/*",
                vec![
                    JsonPointerItem::Key("hello".into()),
                    JsonPointerItem::Wildcard,
                ],
            ),
            ("1234", vec![JsonPointerItem::Number(1234)]),
            (
                "/hello/1234",
                vec![
                    JsonPointerItem::Key("hello".into()),
                    JsonPointerItem::Number(1234),
                ],
            ),
            (
                "/hello/01",
                vec![
                    JsonPointerItem::Key("hello".into()),
                    JsonPointerItem::Key("01".into()),
                ],
            ),
            ("~0~1", vec![JsonPointerItem::Key("~/".into())]),
            (
                "/hello/~0~1",
                vec![
                    JsonPointerItem::Key("hello".into()),
                    JsonPointerItem::Key("~/".into()),
                ],
            ),
            (
                "/hello/1~0~1/*~1~0",
                vec![
                    JsonPointerItem::Key("hello".into()),
                    JsonPointerItem::Key("1~/".into()),
                    JsonPointerItem::Key("*/~".into()),
                ],
            ),
            (
                "/hello/world/*/99",
                vec![
                    JsonPointerItem::Key("hello".into()),
                    JsonPointerItem::Key("world".into()),
                    JsonPointerItem::Wildcard,
                    JsonPointerItem::Number(99),
                ],
            ),
            ("/", vec![JsonPointerItem::Key("".into())]),
            (
                "///",
                vec![
                    JsonPointerItem::Key("".into()),
                    JsonPointerItem::Key("".into()),
                    JsonPointerItem::Key("".into()),
                ],
            ),
            ("", vec![JsonPointerItem::Root]),
            (
                "addresses/1k46/components/0/value",
                vec![
                    JsonPointerItem::Key("addresses".into()),
                    JsonPointerItem::Key("1k46".into()),
                    JsonPointerItem::Key("components".into()),
                    JsonPointerItem::Number(0),
                    JsonPointerItem::Key("value".into()),
                ],
            ),
            (
                "participants/3fa85f64/links/1",
                vec![
                    JsonPointerItem::Key("participants".into()),
                    JsonPointerItem::Key("3fa85f64".into()),
                    JsonPointerItem::Key("links".into()),
                    JsonPointerItem::Number(1),
                ],
            ),
            (
                "2025-03-05T09:00:00/12",
                vec![
                    JsonPointerItem::Key("2025-03-05T09:00:00".into()),
                    JsonPointerItem::Number(12),
                ],
            ),
            (
                "1*/7",
                vec![
                    JsonPointerItem::Key("1*".into()),
                    JsonPointerItem::Number(7),
                ],
            ),
            (
                "keywords/18446744073709551615",
                vec![
                    JsonPointerItem::Key("keywords".into()),
                    JsonPointerItem::Number(u64::MAX),
                ],
            ),
            (
                "keywords/18446744073709551616",
                vec![
                    JsonPointerItem::Key("keywords".into()),
                    JsonPointerItem::Key("18446744073709551616".into()),
                ],
            ),
            (
                "keywords/foo~",
                vec![
                    JsonPointerItem::Key("keywords".into()),
                    JsonPointerItem::Key("foo~".into()),
                ],
            ),
            (
                "keywords/a~2b",
                vec![
                    JsonPointerItem::Key("keywords".into()),
                    JsonPointerItem::Invalid("a~2b".to_string()),
                ],
            ),
            ("a~~0b", vec![JsonPointerItem::Invalid("a~~0b".to_string())]),
            (
                "keywords/a~0~2b/c",
                vec![
                    JsonPointerItem::Key("keywords".into()),
                    JsonPointerItem::Invalid("a~0~2b".to_string()),
                    JsonPointerItem::Key("c".into()),
                ],
            ),
            (
                r"a\b/c\",
                vec![
                    JsonPointerItem::Key(r"a\b".into()),
                    JsonPointerItem::Key(r"c\".into()),
                ],
            ),
            (
                "a~/b",
                vec![
                    JsonPointerItem::Key("a~".into()),
                    JsonPointerItem::Key("b".into()),
                ],
            ),
            ("12~", vec![JsonPointerItem::Key("12~".into())]),
            ("~", vec![JsonPointerItem::Key("~".into())]),
        ] {
            assert_eq!(JsonPointer::parse(input).0, output, "{input}");
        }
    }

    #[test]
    fn json_pointer_parse_promotes_digit() {
        let pointer = JsonPointer::<TestProp>::parse("ids/2");
        assert_eq!(
            pointer.0,
            vec![
                JsonPointerItem::Key(Key::Property(TestProp::Ids)),
                JsonPointerItem::Key(Key::Property(TestProp::Id("2".to_string()))),
            ]
        );

        let pointer = JsonPointer::<TestProp>::parse("ids/abc");
        assert_eq!(
            pointer.0,
            vec![
                JsonPointerItem::Key(Key::Property(TestProp::Ids)),
                JsonPointerItem::Key(Key::Property(TestProp::Id("abc".to_string()))),
            ]
        );

        let pointer = JsonPointer::<TestProp>::parse("other/2");
        assert_eq!(
            pointer.0,
            vec![
                JsonPointerItem::Key(Key::Owned("other".to_string())),
                JsonPointerItem::Number(2),
            ]
        );
    }

    #[test]
    fn json_pointer_parse_leading_zero_is_string() {
        let pointer = JsonPointer::<TestProp>::parse("other/07");
        assert_eq!(
            pointer.0,
            vec![
                JsonPointerItem::Key(Key::Owned("other".to_string())),
                JsonPointerItem::Key(Key::Owned("07".to_string())),
            ]
        );

        let pointer = JsonPointer::<TestProp>::parse("other/00");
        assert_eq!(
            pointer.0,
            vec![
                JsonPointerItem::Key(Key::Owned("other".to_string())),
                JsonPointerItem::Key(Key::Owned("00".to_string())),
            ]
        );

        let pointer = JsonPointer::<TestProp>::parse("other/0");
        assert_eq!(
            pointer.0,
            vec![
                JsonPointerItem::Key(Key::Owned("other".to_string())),
                JsonPointerItem::Number(0),
            ]
        );

        let pointer = JsonPointer::<TestProp>::parse("other/70");
        assert_eq!(
            pointer.0,
            vec![
                JsonPointerItem::Key(Key::Owned("other".to_string())),
                JsonPointerItem::Number(70),
            ]
        );
    }

    fn nested_keys(levels: usize) -> Vec<String> {
        successors(Some(String::from("a/b")), |key| {
            Some(JsonPointer::<Null>::encode([key.as_str(), "x"]))
        })
        .take(levels)
        .collect()
    }

    fn owned(text: &str) -> JsonPointerItem<Nested> {
        JsonPointerItem::Key(Key::Owned(text.to_string()))
    }

    fn expected_nesting(keys: &[String], levels: usize) -> Nested {
        let limit = usize::from(PointerDepth::LIMIT);
        let innermost = match levels
            .checked_sub(limit + 1)
            .and_then(|untyped| keys.get(untyped))
        {
            Some(untyped) => vec![owned(untyped), owned("x")],
            None => vec![owned("a"), owned("b")],
        };
        (1..levels.min(limit)).fold(Nested::Pointer(JsonPointer::new(innermost)), |inner, _| {
            Nested::Pointer(JsonPointer::new(vec![
                JsonPointerItem::Key(Key::Property(inner)),
                owned("x"),
            ]))
        })
    }

    fn nesting(property: &Nested) -> usize {
        match property {
            Nested::Pointer(pointer) => {
                1 + match pointer.first() {
                    Some(JsonPointerItem::Key(Key::Property(inner))) => nesting(inner),
                    _ => 0,
                }
            }
            _ => 0,
        }
    }

    fn only_key(value: &TestValue<'_>) -> String {
        let keys = value
            .as_object()
            .map(|object| {
                object
                    .keys()
                    .map(|key| format!("{key:?}"))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        assert_eq!(keys.len(), 1, "{keys:?}");
        keys.concat()
    }

    #[test]
    fn nested_pointer_keys_stop_at_the_depth_limit() {
        thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(|| {
                let keys = nested_keys(DEEP_LEVELS);
                let limit = usize::from(PointerDepth::LIMIT);
                for (levels, key) in (1..)
                    .zip(&keys)
                    .filter(|(levels, _)| *levels <= limit + 24 || *levels == DEEP_LEVELS)
                {
                    let expected = expected_nesting(&keys, levels);
                    assert_eq!(nesting(&expected), levels.min(limit), "{levels}");
                    let expected_key = format!("{:?}", Key::<Nested>::Property(expected.clone()));
                    let json = format!("{{\"{key}\":1}}");
                    let parsed: TestValue<'_> = Value::parse_json(&json).expect("parses");
                    assert_eq!(only_key(&parsed), expected_key, "{levels}");
                    let parsed: TestValue<'_> = serde_json::from_str(&json).expect("parses");
                    assert_eq!(only_key(&parsed), expected_key, "{levels}");
                    let Nested::Pointer(expected) = expected else {
                        panic!("{levels} levels must parse to a pointer");
                    };
                    assert_eq!(
                        format!("{:?}", JsonPointer::<Nested>::parse(key)),
                        format!("{expected:?}"),
                        "{levels}"
                    );
                    assert_eq!(expected.to_string(), *key, "{levels}");
                }
            })
            .expect("spawns")
            .join()
            .expect("finishes");
    }

    #[test]
    fn parse_nested_refuses_to_go_past_the_depth_limit() {
        let mut depth = PointerDepth::default();
        for _ in 0..PointerDepth::LIMIT {
            let pointer = JsonPointer::<Null>::parse_nested("a/b", depth).expect("below the limit");
            assert_eq!(pointer, JsonPointer::parse("a/b"));
            depth = depth.inner();
        }
        assert_eq!(JsonPointer::<Null>::parse_nested("a/b", depth), None);
    }

    #[test]
    fn deserializes_pointers_with_escaped_solidus() {
        let plain: JsonPointer<Null> = serde_json::from_str(r#""/list/*/id""#).expect("plain");
        let escaped: JsonPointer<Null> =
            serde_json::from_str(r#""\/list\/*\/id""#).expect("escaped solidus must deserialize");

        assert_eq!(plain, escaped);
    }

    #[test]
    fn rfc6901_3_invalid_escapes_do_not_evaluate() {
        use crate::{JsonPointerHandler, Map, Value};

        let object: Value<'_, Null, Null> = Value::Object(Map::from(vec![
            (Key::Owned("a2b".to_string()), Value::Bool(true)),
            (Key::Owned("a~2b".to_string()), Value::Bool(true)),
        ]));
        let pointer = JsonPointer::<Null>::parse("a~2b");

        assert_eq!(
            pointer.as_slice(),
            [JsonPointerItem::Invalid("a~2b".to_string())]
        );
        assert_eq!(pointer.to_string(), "a~2b");

        let mut results = Vec::new();
        object.eval_jptr(pointer.iter(), &mut results);
        assert!(results.is_empty(), "{results:?}");

        let mut patched = object.clone();
        assert!(!patched.patch_jptr(pointer.iter(), Value::Bool(false)));
        assert_eq!(patched, object);
    }

    #[test]
    fn rfc6901_3_invalid_tokens_round_trip_through_display() {
        for pointer in ["a~2b", "keywords/a~2b", "a~~0b", "keywords/a~0~2b/c"] {
            let parsed = JsonPointer::<Null>::parse(pointer);
            assert_eq!(
                parsed.to_string(),
                pointer,
                "RFC 6901 Section 3: a token that matches nothing must not be rewritten into one that matches"
            );
        }

        for pointer in [
            "a~2b",
            "keywords/a~2b",
            "a~~0b",
            "keywords/a~0~2b/c",
            "keywords/foo~",
            "a~/b",
            "keywords/a~0b",
            "keywords/a~1b",
        ] {
            let parsed = JsonPointer::<Null>::parse(pointer);
            assert_eq!(
                JsonPointer::<Null>::parse(&parsed.to_string()).as_slice(),
                parsed.as_slice(),
                "{pointer}"
            );
        }
    }
}
