/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::{JsonPointer, JsonPointerItem, Key, Null, ObjectAsVec, PointerDepth, Property, Value};
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem::discriminant;

const ROUNDS: usize = 10_000;

thread_local! {
    static FAST_HOOKS: Cell<bool> = const { Cell::new(true) };
}

fn with_hooks(fast: bool, test: impl FnOnce()) {
    FAST_HOOKS.with(|hooks| hooks.set(fast));
    test();
    FAST_HOOKS.with(|hooks| hooks.set(true));
}

fn fast_hooks() -> bool {
    FAST_HOOKS.with(Cell::get)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Prop {
    Title,
    Ids,
    Nums,
    Seven,
    Empty,
    Slashed,
    Tilded,
    Accent,
    Id(String),
    Num(u64),
    Ptr(JsonPointer<Prop>),
}

const UNIT_PROPS: [Prop; 8] = [
    Prop::Title,
    Prop::Ids,
    Prop::Nums,
    Prop::Seven,
    Prop::Empty,
    Prop::Slashed,
    Prop::Tilded,
    Prop::Accent,
];

impl Prop {
    fn name(&self) -> Option<&'static str> {
        match self {
            Prop::Title => Some("title"),
            Prop::Ids => Some("ids"),
            Prop::Nums => Some("nums"),
            Prop::Seven => Some("7"),
            Prop::Empty => Some(""),
            Prop::Slashed => Some("a/b"),
            Prop::Tilded => Some("x~y"),
            Prop::Accent => Some("é"),
            Prop::Id(_) | Prop::Num(_) | Prop::Ptr(_) => None,
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        UNIT_PROPS
            .iter()
            .find(|prop| prop.name() == Some(name))
            .cloned()
    }
}

impl Property for Prop {
    fn try_parse(key: Option<&Key<'_, Self>>, value: &str) -> Option<Self> {
        Self::try_parse_nested(key, value, PointerDepth::default())
    }

    fn try_parse_nested(
        key: Option<&Key<'_, Self>>,
        value: &str,
        depth: PointerDepth,
    ) -> Option<Self> {
        match key {
            Some(Key::Property(Prop::Ids)) => Some(Prop::Id(value.to_string())),
            Some(Key::Property(Prop::Nums)) => value.parse().ok().map(Prop::Num),
            None if value.contains('/') && value.len() > 3 => {
                JsonPointer::parse_nested(value, depth).map(Prop::Ptr)
            }
            _ => Prop::from_name(value),
        }
    }

    fn to_cow(&self) -> Cow<'static, str> {
        match self {
            Prop::Id(id) => Cow::Owned(id.clone()),
            Prop::Num(n) => Cow::Owned(n.to_string()),
            Prop::Ptr(pointer) => Cow::Owned(pointer.to_string()),
            unit => Cow::Borrowed(unit.name().unwrap_or_default()),
        }
    }

    fn key_eq(&self, other: &Self) -> bool {
        if !fast_hooks() {
            return self.to_cow() == other.to_cow();
        }
        match (self.name(), other.name()) {
            (Some(_), Some(_)) => discriminant(self) == discriminant(other),
            (Some(name), None) => other.key_eq_str(name),
            (None, Some(name)) => self.key_eq_str(name),
            (None, None) => self.to_cow() == other.to_cow(),
        }
    }

    fn key_eq_str(&self, other: &str) -> bool {
        if !fast_hooks() {
            return self.to_cow() == other;
        }
        match self {
            Prop::Id(id) => id == other,
            Prop::Num(n) => super::NumberKey::default().format(*n) == other,
            Prop::Ptr(pointer) => pointer.to_string() == other,
            unit => unit.name() == Some(other),
        }
    }

    fn key_hash<H: Hasher>(&self, state: &mut H) {
        match self.name() {
            Some(name) if fast_hooks() => name.hash(state),
            _ => self.to_cow().hash(state),
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn chance(&mut self, one_in: u64) -> bool {
        self.next().is_multiple_of(one_in)
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

const PIECES: &[&str] = &[
    "title",
    "ids",
    "nums",
    "7",
    "007",
    "0",
    "00",
    "01",
    "1",
    "12",
    "99",
    "a",
    "b",
    "x~y",
    "a/b",
    "é",
    "日本",
    "🦀",
    "~",
    "~0",
    "~1",
    "~2",
    "~~",
    "~/",
    "/",
    "//",
    "*",
    "**",
    "-",
    " ",
    "\\",
    "\"",
    "\u{0}",
    "18446744073709551615",
    "18446744073709551616",
    "99999999999999999999",
    "2025-03-05T09:00:00",
    "",
];

const RANDOM_CHARS: &[char] = &[
    'a',
    'z',
    'A',
    '0',
    '5',
    '9',
    '~',
    '/',
    '*',
    '-',
    '.',
    '_',
    ' ',
    '\t',
    '\n',
    '\\',
    '"',
    '\u{7f}',
    'é',
    'ß',
    '€',
    '日',
    '🦀',
    '\u{10ffff}',
];

fn random_text(rng: &mut Rng, max_pieces: usize) -> String {
    let mut text = String::new();
    for _ in 0..rng.below(max_pieces + 1) {
        if rng.chance(4) {
            for _ in 0..=rng.below(3) {
                text.push(rng.pick(RANDOM_CHARS));
            }
        } else if rng.chance(20) {
            text.push_str(&rng.next().to_string());
        } else {
            text.push_str(rng.pick(PIECES));
        }
    }
    text
}

fn random_pointer_text(rng: &mut Rng) -> String {
    let mut text = String::new();
    for _ in 0..rng.below(14) {
        if rng.chance(3) {
            text.push('/');
        } else if rng.chance(12) {
            for _ in 0..rng.below(40) {
                text.push(rng.pick(RANDOM_CHARS));
            }
        } else {
            text.push_str(&random_text(rng, 2));
        }
    }
    text
}

fn random_number(rng: &mut Rng) -> u64 {
    match rng.below(8) {
        0 => 0,
        1 => 7,
        2 => 12,
        3 => u64::MAX,
        4 => rng.next() % 100,
        5 => rng.next() >> rng.below(64),
        _ => rng.next(),
    }
}

fn random_prop(rng: &mut Rng, depth: usize) -> Prop {
    match rng.below(14) {
        0..=7 => rng
            .pick(&[0, 1, 2, 3, 4, 5, 6, 7])
            .pipe(|index| UNIT_PROPS[index].clone()),
        8 | 9 => Prop::Id(random_text(rng, 3)),
        10 => Prop::Num(random_number(rng)),
        11 if depth < 2 => Prop::Ptr(random_pointer(rng, depth + 1)),
        _ => Prop::Title,
    }
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

fn random_key(rng: &mut Rng, depth: usize) -> Key<'static, Prop> {
    match rng.below(3) {
        0 => Key::Property(random_prop(rng, depth)),
        1 => Key::Borrowed(if rng.chance(2) {
            rng.pick(PIECES)
        } else {
            leak(random_text(rng, 3))
        }),
        _ => Key::Owned(random_text(rng, 3)),
    }
}

fn related_key(rng: &mut Rng, key: &Key<'static, Prop>) -> Key<'static, Prop> {
    let text = key_text(key);
    match rng.below(6) {
        0 => Key::Borrowed(leak(text)),
        1 => Key::Owned(text),
        2 => Prop::from_name(&text)
            .map(Key::Property)
            .unwrap_or_else(|| Key::Property(Prop::Id(text))),
        3 => text
            .parse()
            .ok()
            .map(|n| Key::Property(Prop::Num(n)))
            .unwrap_or(Key::Owned(text)),
        4 => Key::Property(Prop::Ptr(JsonPointer::parse(&text))),
        _ => random_key(rng, 1),
    }
}

fn key_text(key: &Key<'_, Prop>) -> String {
    match key {
        Key::Property(prop) => prop.to_cow().into_owned(),
        Key::Borrowed(text) => text.to_string(),
        Key::Owned(text) => text.clone(),
    }
}

fn random_item(rng: &mut Rng, depth: usize) -> JsonPointerItem<Prop> {
    match rng.below(8) {
        0 => JsonPointerItem::Root,
        1 => JsonPointerItem::Wildcard,
        2 => JsonPointerItem::Invalid(random_text(rng, 3)),
        3 => JsonPointerItem::Number(random_number(rng)),
        _ => JsonPointerItem::Key(random_key(rng, depth)),
    }
}

fn random_pointer(rng: &mut Rng, depth: usize) -> JsonPointer<Prop> {
    if rng.chance(2) {
        JsonPointer::parse(&random_pointer_text(rng))
    } else {
        JsonPointer::new((0..rng.below(6)).map(|_| random_item(rng, depth)).collect())
    }
}

fn hash_of(key: &Key<'_, Prop>) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

fn text_hash_of(key: &Key<'_, Prop>) -> u64 {
    let mut hasher = DefaultHasher::new();
    key_text(key).as_str().hash(&mut hasher);
    hasher.finish()
}

#[test]
fn key_eq_and_hash_follow_the_text() {
    for fast in [true, false] {
        with_hooks(fast, || {
            let mut rng = Rng::new(0x6b65_795f_6571_7561);
            for _ in 0..ROUNDS {
                let a = random_key(&mut rng, 0);
                let b = if rng.chance(2) {
                    related_key(&mut rng, &a)
                } else {
                    random_key(&mut rng, 0)
                };
                let expected = key_text(&a) == key_text(&b);
                assert_eq!(a == b, expected, "{a:?} == {b:?}");
                assert_eq!(b == a, expected, "{b:?} == {a:?}");
                assert_eq!(hash_of(&a), text_hash_of(&a), "hash {a:?}");
                assert_eq!(hash_of(&b), text_hash_of(&b), "hash {b:?}");

                let text = key_text(&b);
                assert_eq!(
                    a == text.as_str(),
                    key_text(&a) == text,
                    "{a:?} == {text:?}"
                );
            }
        });
    }
}

#[test]
fn object_lookups_match_a_linear_search_by_text() {
    for fast in [true, false] {
        with_hooks(fast, || {
            let mut rng = Rng::new(0x6f62_6a65_6374_5f6f);
            for _ in 0..ROUNDS / 4 {
                let mut object = ObjectAsVec::<'static, Prop, Null>::new();
                for index in 0..rng.below(12) {
                    let key = if index > 0 && rng.chance(3) {
                        let existing = rng.below(object.0.len());
                        related_key(&mut rng, &object.0[existing].0)
                    } else {
                        random_key(&mut rng, 0)
                    };
                    object.insert_unchecked(key, Value::Number((index as u64).into()));
                }
                let key = if !object.0.is_empty() && rng.chance(2) {
                    let index = rng.below(object.0.len());
                    related_key(&mut rng, &object.0[index].0)
                } else {
                    random_key(&mut rng, 0)
                };
                let text = key_text(&key);
                let position = object.0.iter().position(|(k, _)| key_text(k) == text);
                let expected = position.map(|index| &object.0[index]);

                assert_eq!(
                    object.get(&key).map(|value| format!("{value:?}")),
                    expected.map(|(_, value)| format!("{value:?}")),
                    "{key:?} in {object:?}"
                );
                assert_eq!(
                    format!("{:?}", object.get_key_value(&key)),
                    format!("{:?}", expected.map(|(k, v)| (k, v))),
                    "{key:?} in {object:?}"
                );
                assert_eq!(object.contains_key(&key), position.is_some());
                let other = random_key(&mut rng, 0);
                let other_text = key_text(&other);
                assert_eq!(
                    object.contains_any_key(&[other.clone(), key.clone()]),
                    position.is_some() || object.0.iter().any(|(k, _)| key_text(k) == other_text)
                );
                let value = Value::Number((rng.below(12) as u64).into());
                assert_eq!(
                    object.contains_key_value(&key, &value),
                    object
                        .0
                        .iter()
                        .any(|(k, v)| key_text(k) == text && v == &value)
                );

                let mut mutated = object.clone();
                if let Some(value) = mutated.get_mut(&key) {
                    *value = Value::Bool(true);
                }
                for (index, (member, value)) in mutated.0.iter().enumerate() {
                    assert_eq!(
                        value == &Value::Bool(true),
                        Some(index) == position,
                        "{member:?} after get_mut {key:?}"
                    );
                }
            }
        });
    }
}

#[test]
fn parse_classifies_tokens() {
    for (text, expected, untyped) in [
        ("", "[Root]", "[Root]"),
        ("/", "[Key(Property(Empty))]", r#"[Key(Borrowed(""))]"#),
        (
            "//",
            "[Key(Property(Empty)), Key(Property(Empty))]",
            r#"[Key(Borrowed("")), Key(Borrowed(""))]"#,
        ),
        ("/a", r#"[Key(Owned("a"))]"#, r#"[Key(Owned("a"))]"#),
        (
            "a//b",
            r#"[Key(Owned("a")), Key(Property(Empty)), Key(Owned("b"))]"#,
            r#"[Key(Owned("a")), Key(Borrowed("")), Key(Owned("b"))]"#,
        ),
        (
            "/a/b/",
            r#"[Key(Owned("a")), Key(Owned("b")), Key(Property(Empty))]"#,
            r#"[Key(Owned("a")), Key(Owned("b")), Key(Borrowed(""))]"#,
        ),
        ("7", "[Key(Property(Seven))]", "[Number(7)]"),
        ("007", r#"[Key(Owned("007"))]"#, r#"[Key(Owned("007"))]"#),
        ("0", "[Number(0)]", "[Number(0)]"),
        (
            "18446744073709551615",
            "[Number(18446744073709551615)]",
            "[Number(18446744073709551615)]",
        ),
        (
            "18446744073709551616",
            r#"[Key(Owned("18446744073709551616"))]"#,
            r#"[Key(Owned("18446744073709551616"))]"#,
        ),
        ("*", "[Wildcard]", "[Wildcard]"),
        ("**", r#"[Key(Owned("**"))]"#, r#"[Key(Owned("**"))]"#),
        ("*/1", "[Wildcard, Number(1)]", "[Wildcard, Number(1)]"),
        ("~0~1", r#"[Key(Owned("~/"))]"#, r#"[Key(Owned("~/"))]"#),
        (
            "a~0b~1c",
            r#"[Key(Property(Ptr(JsonPointer([Invalid("a~b"), Key(Owned("c"))]))))]"#,
            r#"[Key(Owned("a~b/c"))]"#,
        ),
        ("~2", r#"[Invalid("~2")]"#, r#"[Invalid("~2")]"#),
        ("a~", r#"[Key(Owned("a~"))]"#, r#"[Key(Owned("a~"))]"#),
        ("~~", r#"[Invalid("~~")]"#, r#"[Invalid("~~")]"#),
        (
            "ids/x/7",
            r#"[Key(Property(Ids)), Key(Property(Id("x"))), Key(Property(Seven))]"#,
            r#"[Key(Owned("ids")), Key(Owned("x")), Number(7)]"#,
        ),
        (
            "nums/12/nums",
            "[Key(Property(Nums)), Key(Property(Num(12))), Key(Property(Nums))]",
            r#"[Key(Owned("nums")), Number(12), Key(Owned("nums"))]"#,
        ),
        (
            "nums/007",
            "[Key(Property(Nums)), Key(Property(Num(7)))]",
            r#"[Key(Owned("nums")), Key(Owned("007"))]"#,
        ),
        (
            "\u{e9}/\u{65e5}\u{672c}",
            "[Key(Property(Accent)), Key(Owned(\"\u{65e5}\u{672c}\"))]",
            "[Key(Owned(\"\u{e9}\")), Key(Owned(\"\u{65e5}\u{672c}\"))]",
        ),
        (
            "a/b~1c/d",
            r#"[Key(Owned("a")), Key(Owned("b/c")), Key(Owned("d"))]"#,
            r#"[Key(Owned("a")), Key(Owned("b/c")), Key(Owned("d"))]"#,
        ),
    ] {
        assert_eq!(
            format!("{:?}", JsonPointer::<Prop>::parse(text).0),
            expected,
            "{text:?}"
        );
        assert_eq!(
            format!("{:?}", JsonPointer::<Null>::parse(text).0),
            untyped,
            "{text:?}"
        );
    }
}

#[test]
fn encode_escapes_tildes_and_slashes() {
    for (items, expected) in [
        (vec![], ""),
        (vec![""], ""),
        (vec!["a"], "a"),
        (vec!["a/b", "~", ""], "a~1b/~0/"),
        (vec!["~0", "~1", "/~/"], "~00/~01/~1~0~1"),
        (vec!["\u{e9}", "x/\u{65e5}~"], "\u{e9}/x~1\u{65e5}~0"),
        (vec!["", "", ""], "//"),
    ] {
        assert_eq!(JsonPointer::<Prop>::encode(&items), expected, "{items:?}");
        assert_eq!(
            JsonPointer::<Prop>::encode(items.iter().map(|item| item.to_string())),
            expected,
            "{items:?}"
        );
    }
}

#[test]
fn display_writes_escaped_runs() {
    for (pointer, expected) in [
        (JsonPointer::<Prop>::new(vec![]), ""),
        (JsonPointer::new(vec![JsonPointerItem::Root]), ""),
        (
            JsonPointer::new(vec![JsonPointerItem::Wildcard, JsonPointerItem::Number(7)]),
            "*/7",
        ),
        (
            JsonPointer::new(vec![
                JsonPointerItem::Key(Key::Borrowed("a/b")),
                JsonPointerItem::Invalid("x~y".to_string()),
                JsonPointerItem::Key(Key::Property(Prop::Tilded)),
            ]),
            "a~1b/x~y/x~0y",
        ),
        (
            JsonPointer::new(vec![
                JsonPointerItem::Root,
                JsonPointerItem::Key(Key::Property(Prop::Slashed)),
                JsonPointerItem::Number(u64::MAX),
                JsonPointerItem::Key(Key::Owned(String::new())),
            ]),
            "/a~1b/18446744073709551615/",
        ),
        (
            JsonPointer::new(vec![JsonPointerItem::Key(Key::Property(Prop::Ptr(
                JsonPointer::parse("ids/q/7"),
            )))]),
            "ids~1q~17",
        ),
    ] {
        assert_eq!(pointer.to_string(), expected, "{pointer:?}");
        assert_eq!(format!("{pointer:>40.3}"), expected, "{pointer:?}");
        assert_eq!(
            serde_json::to_string(&pointer).expect("serializable pointer"),
            serde_json::to_string(expected).expect("serializable string"),
        );
    }
}
