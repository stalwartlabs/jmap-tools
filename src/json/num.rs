/*
 * SPDX-FileCopyrightText: 2021 Pascal Seitz <pascal.seitz@gmail.com>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use core::hash::{Hash, Hasher};

/// Represents a JSON number, whether integer or floating point.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Number {
    pub(crate) n: N,
}

impl From<N> for Number {
    fn from(n: N) -> Self {
        Self { n }
    }
}

#[derive(Copy, Clone)]
pub(crate) enum N {
    PosInt(u64),
    /// Always less than zero.
    NegInt(i64),
    Float(f64),
}

impl Number {
    /// If the `Number` is an integer, represent it as i64 if possible. Returns
    /// None otherwise.
    pub fn as_u64(&self) -> Option<u64> {
        match self.n {
            N::PosInt(v) => Some(v),
            _ => None,
        }
    }
    /// If the `Number` is an integer, represent it as u64 if possible. Returns
    /// None otherwise.
    pub fn as_i64(&self) -> Option<i64> {
        match self.n {
            N::PosInt(n) => {
                if n <= i64::MAX as u64 {
                    Some(n as i64)
                } else {
                    None
                }
            }
            N::NegInt(v) => Some(v),
            _ => None,
        }
    }

    /// Represents the number as f64 if possible. Returns None otherwise.
    pub fn as_f64(&self) -> Option<f64> {
        match self.n {
            N::PosInt(n) => Some(n as f64),
            N::NegInt(n) => Some(n as f64),
            N::Float(n) => Some(n),
        }
    }

    /// Returns true if the `Number` is a f64.
    pub fn is_f64(&self) -> bool {
        matches!(self.n, N::Float(_))
    }

    /// Returns true if the `Number` is a u64.
    pub fn is_u64(&self) -> bool {
        matches!(self.n, N::PosInt(_))
    }

    /// Returns true if the `Number` is an integer between `i64::MIN` and
    /// `i64::MAX`.
    pub fn is_i64(&self) -> bool {
        match self.n {
            N::PosInt(v) => v <= i64::MAX as u64,
            N::NegInt(_) => true,
            N::Float(_) => false,
        }
    }

    pub fn cast_to_i64(self) -> i64 {
        match self.n {
            N::PosInt(v) => i64::try_from(v).unwrap_or(i64::MAX),
            N::NegInt(v) => v,
            N::Float(v) => v as i64,
        }
    }

    pub fn cast_to_u64(self) -> u64 {
        match self.n {
            N::PosInt(v) => v,
            N::NegInt(_) => 0,
            N::Float(v) => v as u64,
        }
    }

    pub fn try_cast_to_i64(self) -> Result<i64, f64> {
        match self.n {
            N::PosInt(v) => i64::try_from(v).map_err(|_| v as f64),
            N::NegInt(v) => Ok(v),
            N::Float(v) => Err(v),
        }
    }

    pub(crate) fn into_json_value(self) -> serde_json::Value {
        match self.n {
            N::PosInt(n) => serde_json::Value::Number(n.into()),
            N::NegInt(n) => serde_json::Value::Number(n.into()),
            N::Float(n) => serde_json::value::Number::from_f64(n)
                .map_or(serde_json::Value::Null, serde_json::Value::Number),
        }
    }
}

impl PartialEq for N {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (N::PosInt(a), N::PosInt(b)) => a == b,
            (N::NegInt(a), N::NegInt(b)) => a == b,
            (N::Float(a), N::Float(b)) => a == b || (a.is_nan() && b.is_nan()),
            _ => false,
        }
    }
}

impl Eq for N {}

impl Hash for N {
    fn hash<H: Hasher>(&self, h: &mut H) {
        match *self {
            N::PosInt(i) => i.hash(h),
            N::NegInt(i) => i.hash(h),
            N::Float(f) => {
                let bits = if f == 0.0 {
                    0.0f64.to_bits()
                } else if f.is_nan() {
                    f64::NAN.to_bits()
                } else {
                    f.to_bits()
                };
                bits.hash(h);
            }
        }
    }
}
impl From<u64> for Number {
    fn from(val: u64) -> Self {
        Self { n: N::PosInt(val) }
    }
}

impl From<i64> for Number {
    fn from(val: i64) -> Self {
        let n = match u64::try_from(val) {
            Ok(val) => N::PosInt(val),
            Err(_) => N::NegInt(val),
        };
        Self { n }
    }
}

impl From<f64> for Number {
    fn from(val: f64) -> Self {
        Self { n: N::Float(val) }
    }
}

impl From<usize> for Number {
    fn from(val: usize) -> Self {
        Self {
            n: N::PosInt(val as u64),
        }
    }
}

impl From<isize> for Number {
    fn from(val: isize) -> Self {
        Self::from(val as i64)
    }
}

impl From<u32> for Number {
    fn from(val: u32) -> Self {
        Self {
            n: N::PosInt(val as u64),
        }
    }
}

impl From<i32> for Number {
    fn from(val: i32) -> Self {
        Self::from(i64::from(val))
    }
}

impl TryFrom<Number> for serde_json::value::Number {
    type Error = f64;

    fn try_from(num: Number) -> Result<Self, Self::Error> {
        match num.n {
            N::PosInt(n) => Ok(n.into()),
            N::NegInt(n) => Ok(n.into()),
            N::Float(n) => serde_json::value::Number::from_f64(n).ok_or(n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{N, Number};
    use crate::{Null, Value};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    #[test]
    fn signed_and_unsigned_integers_compare_equal() {
        assert!(Number::from(7i64) == Number::from(7u64));
        assert!(Number::from(0i64) == Number::from(0u64));
        assert!(Number::from(5i32) == Number::from(5u32));
        assert!(Number::from(9isize) == Number::from(9usize));
        assert!(matches!(Number::from(7i64).n, N::PosInt(7)));
        assert!(matches!(Number::from(0i32).n, N::PosInt(0)));
        assert!(matches!(Number::from(i64::MAX).n, N::PosInt(v) if v == i64::MAX as u64));
    }

    #[test]
    fn negative_integers_are_neg_int() {
        let number = Number::from(-3i64);
        assert!(matches!(number.n, N::NegInt(-3)));
        assert!(matches!(Number::from(-3i32).n, N::NegInt(-3)));
        assert!(matches!(Number::from(i64::MIN).n, N::NegInt(i64::MIN)));

        let parsed: Value<'_, Null, Null> = serde_json::from_str("-3").expect("valid json");
        assert_eq!(parsed, Value::Number(number));
    }

    #[test]
    fn parsed_value_equals_value_from_i64() {
        let parsed: Value<'_, Null, Null> = serde_json::from_str("7").expect("valid json");
        assert_eq!(parsed, Value::Number(7i64.into()));
        let parsed: Value<'_, Null, Null> = serde_json::from_str("0").expect("valid json");
        assert_eq!(parsed, Value::Number(0i64.into()));
    }

    #[test]
    fn serialization_is_unchanged() {
        let serialize =
            |number: Number| serde_json::to_string(&number).expect("serializable number");
        assert_eq!(serialize(Number::from(7i64)), "7");
        assert_eq!(serialize(Number::from(0i64)), "0");
        assert_eq!(serialize(Number::from(5i32)), "5");
        assert_eq!(serialize(Number::from(-3i64)), "-3");
        assert_eq!(serialize(Number::from(i64::MAX)), i64::MAX.to_string());
        assert_eq!(serialize(Number::from(i64::MIN)), i64::MIN.to_string());
    }

    #[test]
    fn hash_is_consistent_with_eq() {
        let hash = |number: Number| {
            let mut hasher = DefaultHasher::new();
            number.hash(&mut hasher);
            hasher.finish()
        };
        assert_eq!(hash(Number::from(7i64)), hash(Number::from(7u64)));
    }

    #[test]
    fn non_finite_floats_are_consistent() {
        let hash = |number: Number| {
            let mut hasher = DefaultHasher::new();
            number.hash(&mut hasher);
            hasher.finish()
        };
        let nan = Number::from(f64::NAN);
        let other_nan = Number::from(f64::NAN);
        let negative_nan = Number::from(-f64::NAN);
        let payload_nan = Number::from(f64::from_bits(f64::NAN.to_bits() | 1));
        assert!(nan == other_nan);
        assert!(nan == negative_nan);
        assert!(nan == payload_nan);
        assert_eq!(hash(nan), hash(negative_nan));
        assert_eq!(hash(nan), hash(payload_nan));
        assert!(nan != Number::from(f64::INFINITY));
        assert!(nan != Number::from(0.0f64));
        assert!(Number::from(f64::INFINITY) == Number::from(f64::INFINITY));
        assert!(Number::from(f64::INFINITY) != Number::from(f64::NEG_INFINITY));
        assert!(Number::from(0.0f64) == Number::from(-0.0f64));
        assert_eq!(hash(Number::from(0.0f64)), hash(Number::from(-0.0f64)));

        let value: Value<'_, Null, Null> = Value::Array(vec![Value::Number(nan)]);
        assert_eq!(value, value.clone());
    }

    #[test]
    fn casts_do_not_wrap_or_flip_sign() {
        assert_eq!(Number::from(u64::MAX).cast_to_i64(), i64::MAX);
        assert_eq!(Number::from(i64::MAX).cast_to_i64(), i64::MAX);
        assert_eq!(Number::from(-5i64).cast_to_i64(), -5);
        assert_eq!(Number::from(-5i64).cast_to_u64(), 0);
        assert_eq!(Number::from(5u64).cast_to_u64(), 5);

        assert_eq!(
            Number::from(u64::MAX).try_cast_to_i64(),
            Err(u64::MAX as f64)
        );
        assert_eq!(Number::from(i64::MAX).try_cast_to_i64(), Ok(i64::MAX));
        assert_eq!(Number::from(-5i64).try_cast_to_i64(), Ok(-5));
        assert_eq!(Number::from(1.5f64).try_cast_to_i64(), Err(1.5));
    }

    #[test]
    fn rfc8259_6_non_finite_floats_do_not_convert_to_json_numbers() {
        for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                serde_json::value::Number::try_from(Number::from(number)).map_err(f64::is_finite),
                Err(false),
                "RFC 8259 Section 6: JSON numbers cannot express Infinity or NaN"
            );
        }

        assert_eq!(
            serde_json::value::Number::try_from(Number::from(1.5f64)),
            serde_json::value::Number::from_f64(1.5).ok_or(1.5)
        );
        assert_eq!(
            serde_json::value::Number::try_from(Number::from(-2i64)).map(|n| n.to_string()),
            Ok("-2".to_string())
        );
        assert_eq!(
            serde_json::value::Number::try_from(Number::from(u64::MAX)).map(|n| n.to_string()),
            Ok(u64::MAX.to_string())
        );
    }

    #[test]
    fn non_finite_floats_display_as_null() {
        for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let value: Value<'_, Null, Null> = Value::Number(Number::from(number));
            assert_eq!(value.to_string(), "null");
            assert_eq!(serde_json::Value::from(&value), serde_json::Value::Null);
            assert_eq!(serde_json::to_string(&value).expect("serializable"), "null");
        }
        let value: Value<'_, Null, Null> = Value::Number(Number::from(1.5f64));
        assert_eq!(value.to_string(), "1.5");
        let value: Value<'_, Null, Null> = Value::Number(Number::from(-2i64));
        assert_eq!(value.to_string(), "-2");
    }
}
