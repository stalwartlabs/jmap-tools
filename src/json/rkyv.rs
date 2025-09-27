/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::{Element, Property, Value};
use rkyv::{
    option::ArchivedOption,
    rend::{u32_le, u64_le},
    string::ArchivedString,
};

impl<'ctx, P: Property, E: Element, T> From<&ArchivedOption<T>> for Value<'ctx, P, E>
where
    for<'x> &'x T: Into<Value<'ctx, P, E>>,
{
    fn from(value: &ArchivedOption<T>) -> Self {
        match value {
            ArchivedOption::Some(value) => value.into(),
            ArchivedOption::None => Value::Null,
        }
    }
}

impl<'ctx, P: Property, E: Element> From<&ArchivedString> for Value<'ctx, P, E> {
    fn from(value: &ArchivedString) -> Self {
        Value::Str(value.to_string().into())
    }
}

impl<'ctx, P: Property, E: Element> From<&u32_le> for Value<'ctx, P, E> {
    fn from(value: &u32_le) -> Self {
        Value::Number(u32::from(value).into())
    }
}

impl<'ctx, P: Property, E: Element> From<&u64_le> for Value<'ctx, P, E> {
    fn from(value: &u64_le) -> Self {
        Value::Number(u64::from(value).into())
    }
}
