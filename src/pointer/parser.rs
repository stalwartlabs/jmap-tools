/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::{JsonPointer, JsonPointerItem};
use std::fmt::Display;

enum TokenType {
    Unknown,
    Number,
    String,
    Wildcard,
    Escaped,
}

struct State {
    num: u64,
    buf: Vec<u8>,
    token: TokenType,
    start_pos: usize,
    path: Vec<JsonPointerItem>,
}

impl JsonPointer {
    pub fn parse(value: &str) -> Self {
        let mut state = State {
            num: 0,
            buf: Vec::new(),
            token: TokenType::Unknown,
            start_pos: 0,
            path: Vec::new(),
        };
        let mut iter = value.as_bytes().iter().enumerate();

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
                    state.process();
                    state.token = TokenType::Unknown;
                    state.start_pos = pos + 1;
                }
                (_, _) => {
                    if matches!(&state.token, TokenType::Number | TokenType::Wildcard)
                        && pos > state.start_pos
                    {
                        state.buf.extend_from_slice(
                            value
                                .as_bytes()
                                .get(state.start_pos..pos)
                                .unwrap_or_default(),
                        );
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

        state.process();

        if state.path.is_empty() {
            state.path.push(JsonPointerItem::Root);
        }

        JsonPointer(state.path)
    }
}

impl State {
    pub fn process(&mut self) {
        match self.token {
            TokenType::String => {
                self.path.push(JsonPointerItem::String(
                    String::from_utf8(std::mem::take(&mut self.buf)).unwrap(),
                ));
            }
            TokenType::Number => {
                self.path.push(JsonPointerItem::Number(self.num));
                self.num = 0;
            }
            TokenType::Wildcard => {
                self.path.push(JsonPointerItem::Wildcard);
            }
            TokenType::Unknown if self.start_pos > 0 => {
                self.path.push(JsonPointerItem::String(String::new()));
            }
            _ => (),
        }
    }
}

impl Display for JsonPointer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, ptr) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, "/")?;
            }
            write!(f, "{}", ptr)?;
        }
        Ok(())
    }
}

impl Display for JsonPointerItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonPointerItem::Root => write!(f, "/"),
            JsonPointerItem::Wildcard => write!(f, "*"),
            JsonPointerItem::String(s) => write!(f, "{}", s),
            JsonPointerItem::Number(n) => write!(f, "{}", n),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{JsonPointer, JsonPointerItem};

    #[test]
    fn json_pointer_parse() {
        for (input, output) in vec![
            ("hello", vec![JsonPointerItem::String("hello".to_string())]),
            ("9a", vec![JsonPointerItem::String("9a".to_string())]),
            ("a9", vec![JsonPointerItem::String("a9".to_string())]),
            ("*a", vec![JsonPointerItem::String("*a".to_string())]),
            (
                "/hello/world",
                vec![
                    JsonPointerItem::String("hello".to_string()),
                    JsonPointerItem::String("world".to_string()),
                ],
            ),
            ("*", vec![JsonPointerItem::Wildcard]),
            (
                "/hello/*",
                vec![
                    JsonPointerItem::String("hello".to_string()),
                    JsonPointerItem::Wildcard,
                ],
            ),
            ("1234", vec![JsonPointerItem::Number(1234)]),
            (
                "/hello/1234",
                vec![
                    JsonPointerItem::String("hello".to_string()),
                    JsonPointerItem::Number(1234),
                ],
            ),
            ("~0~1", vec![JsonPointerItem::String("~/".to_string())]),
            (
                "/hello/~0~1",
                vec![
                    JsonPointerItem::String("hello".to_string()),
                    JsonPointerItem::String("~/".to_string()),
                ],
            ),
            (
                "/hello/1~0~1/*~1~0",
                vec![
                    JsonPointerItem::String("hello".to_string()),
                    JsonPointerItem::String("1~/".to_string()),
                    JsonPointerItem::String("*/~".to_string()),
                ],
            ),
            (
                "/hello/world/*/99",
                vec![
                    JsonPointerItem::String("hello".to_string()),
                    JsonPointerItem::String("world".to_string()),
                    JsonPointerItem::Wildcard,
                    JsonPointerItem::Number(99),
                ],
            ),
            ("/", vec![JsonPointerItem::String("".to_string())]),
            (
                "///",
                vec![
                    JsonPointerItem::String("".to_string()),
                    JsonPointerItem::String("".to_string()),
                    JsonPointerItem::String("".to_string()),
                ],
            ),
            ("", vec![JsonPointerItem::Root]),
        ] {
            assert_eq!(JsonPointer::parse(input).0, output, "{input}");
        }
    }
}
