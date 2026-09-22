/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::{JsonPointer, JsonPointerItem, Key, Property};
use std::borrow::Cow;

enum TokenType {
    Unknown,
    Number,
    String,
    Wildcard,
    Escaped,
    Invalid,
}

struct State<P: Property> {
    buf: Vec<u8>,
    token: TokenType,
    start_pos: usize,
    path: Vec<JsonPointerItem<P>>,
}

impl<P: Property> JsonPointer<P> {
    pub fn parse(value: &str) -> Self {
        let mut state = State {
            buf: Vec::new(),
            token: TokenType::Unknown,
            start_pos: 0,
            path: Vec::new(),
        };
        let value = value.as_bytes();

        for (pos, &ch) in value.iter().enumerate() {
            match (ch, &state.token) {
                (b'0'..=b'9', TokenType::Unknown | TokenType::Number) => {
                    state.token = TokenType::Number;
                }
                (b'*', TokenType::Unknown) => {
                    state.token = TokenType::Wildcard;
                }
                (b'0', TokenType::Escaped) => {
                    state.buf.push(b'~');
                    state.token = TokenType::String;
                }
                (b'1', TokenType::Escaped) => {
                    state.buf.push(b'/');
                    state.token = TokenType::String;
                }
                (b'/', _) => {
                    state.process(&value[state.start_pos..pos]);
                    state.token = TokenType::Unknown;
                    state.start_pos = pos + 1;
                }
                (_, TokenType::Escaped | TokenType::Invalid) => {
                    state.token = TokenType::Invalid;
                }
                (_, _) => {
                    if matches!(&state.token, TokenType::Number | TokenType::Wildcard)
                        && pos > state.start_pos
                    {
                        state
                            .buf
                            .extend_from_slice(value.get(state.start_pos..pos).unwrap_or_default());
                    }

                    state.token = match ch {
                        b'~' => TokenType::Escaped,
                        _ => {
                            state.buf.push(ch);
                            TokenType::String
                        }
                    };
                }
            }
        }

        state.process(value.get(state.start_pos..).unwrap_or_default());

        if state.path.is_empty() {
            state.path.push(JsonPointerItem::Root);
        }

        JsonPointer(state.path)
    }
}

impl<P: Property> State<P> {
    pub fn process(&mut self, token_bytes: &[u8]) {
        match self.token {
            TokenType::String | TokenType::Escaped => {
                if matches!(self.token, TokenType::Escaped) {
                    self.buf.push(b'~');
                }
                let item = std::str::from_utf8(&self.buf).unwrap_or_default();
                match P::try_parse(self.path.last().and_then(|item| item.as_key()), item) {
                    Some(prop) => {
                        self.path.push(JsonPointerItem::Key(Key::Property(prop)));
                    }
                    None => {
                        self.path
                            .push(JsonPointerItem::Key(Key::Owned(item.to_string())));
                    }
                }

                self.buf.clear();
            }
            TokenType::Number => {
                let item = std::str::from_utf8(token_bytes).unwrap_or_default();
                let token =
                    match P::try_parse(self.path.last().and_then(|item| item.as_key()), item) {
                        Some(prop) => JsonPointerItem::Key(Key::Property(prop)),
                        None => match item.parse::<u64>() {
                            Ok(index) if item == "0" || !item.starts_with('0') => {
                                JsonPointerItem::Number(index)
                            }
                            _ => JsonPointerItem::Key(Key::Owned(item.to_string())),
                        },
                    };
                self.path.push(token);
            }
            TokenType::Wildcard => {
                self.path.push(JsonPointerItem::Wildcard);
            }
            TokenType::Invalid => {
                self.buf.clear();
                self.path.push(JsonPointerItem::Invalid(
                    String::from_utf8_lossy(token_bytes).into_owned(),
                ));
            }
            TokenType::Unknown if self.start_pos > 0 => {
                self.path.push(JsonPointerItem::Key("".into()));
            }
            _ => (),
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
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {

    use super::{JsonPointer, JsonPointerItem};
    use crate::{Key, Null, Property};
    use std::borrow::Cow;

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
