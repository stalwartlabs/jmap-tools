/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use jmap_tools::{Element, Key, Property};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Name {
    Id,
    Title,
    Items,
    Kind,
    Count,
}

impl Name {
    fn from_text(text: &str) -> Option<Self> {
        match text {
            "id" => Some(Name::Id),
            "title" => Some(Name::Title),
            "items" => Some(Name::Items),
            "kind" => Some(Name::Kind),
            "count" => Some(Name::Count),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Name::Id => "id",
            Name::Title => "title",
            Name::Items => "items",
            Name::Kind => "kind",
            Name::Count => "count",
        }
    }
}

impl Property for Name {
    fn try_parse(key: Option<&Key<'_, Self>>, value: &str) -> Option<Self> {
        match (key, Name::from_text(value)?) {
            (Some(Key::Property(Name::Items)), Name::Kind) => Some(Name::Kind),
            (_, Name::Kind) => None,
            (_, name) => Some(name),
        }
    }

    fn to_cow(&self) -> Cow<'static, str> {
        Cow::Borrowed(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    Alpha,
    Beta,
}

impl Element for Tag {
    type Property = Name;

    fn try_parse<P>(key: &Key<'_, Name>, value: &str) -> Option<Self> {
        match (key, value) {
            (Key::Property(Name::Kind), "alpha") => Some(Tag::Alpha),
            (Key::Property(Name::Kind), "beta") => Some(Tag::Beta),
            _ => None,
        }
    }

    fn to_cow(&self) -> Cow<'static, str> {
        Cow::Borrowed(match self {
            Tag::Alpha => "alpha",
            Tag::Beta => "beta",
        })
    }
}
