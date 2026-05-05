/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::{JsonPointer, JsonPointerItem, Key, Property};

enum TokenType {
    Unknown,
    Number,
    String,
    Wildcard,
    Escaped,
}

struct State<P: Property> {
    num: u64,
    buf: Vec<u8>,
    token: TokenType,
    start_pos: usize,
    path: Vec<JsonPointerItem<P>>,
}

impl<P: Property> JsonPointer<P> {
    pub fn parse(value: &str) -> Self {
        let mut state = State {
            num: 0,
            buf: Vec::new(),
            token: TokenType::Unknown,
            start_pos: 0,
            path: Vec::new(),
        };
        let value = value.as_bytes();
        let mut iter = value.iter().enumerate();

        while let Some((pos, &ch)) = iter.next() {
            match (ch, &state.token) {
                (b'0'..=b'9', TokenType::Unknown | TokenType::Number) => {
                    state.num = state
                        .num
                        .saturating_mul(10)
                        .saturating_add((ch - b'0') as u64);
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
                (_, _) => {
                    if matches!(&state.token, TokenType::Number | TokenType::Wildcard)
                        && pos > state.start_pos
                    {
                        state
                            .buf
                            .extend_from_slice(value.get(state.start_pos..pos).unwrap_or_default());
                    }

                    state.token = match ch {
                        b'~' if !matches!(&state.token, TokenType::Escaped) => TokenType::Escaped,
                        b'\\' => {
                            state
                                .buf
                                .push(iter.next().map(|(_, &ch)| ch).unwrap_or(b'\\'));
                            TokenType::String
                        }
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
            TokenType::String => {
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
                match P::try_parse(self.path.last().and_then(|item| item.as_key()), item) {
                    Some(prop) => {
                        self.path.push(JsonPointerItem::Key(Key::Property(prop)));
                    }
                    None => {
                        self.path.push(JsonPointerItem::Number(self.num));
                    }
                }
                self.num = 0;
            }
            TokenType::Wildcard => {
                self.path.push(JsonPointerItem::Wildcard);
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
        <&str>::deserialize(deserializer).map(|s| JsonPointer::parse(s))
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
