use crate::error::InvalidNumberError;
use std::cmp::Ordering;

/// Represents a YAML number, whether integer or floating point.
#[derive(Clone, PartialEq, PartialOrd)]
pub struct Number(pub N);

// "N" is a prefix of "NegInt"... this is a false positive.
// https://github.com/Manishearth/rust-clippy/issues/1241
#[allow(clippy::enum_variant_names)]
#[derive(Copy, Clone)]
pub enum N {
    PosInt(u64),
    /// Always less than zero.
    NegInt(i64),
    /// May be infinite or NaN.
    Float(f64),
}

impl std::fmt::Display for N {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PosInt(n) => std::fmt::Display::fmt(n, f),
            Self::NegInt(n) => std::fmt::Display::fmt(n, f),
            Self::Float(n) => std::fmt::Display::fmt(n, f),
        }
    }
}

impl From<N> for Number {
    fn from(value: N) -> Self {
        Number(value)
    }
}

impl Number {
    /// Returns true if the `Number` is an integer between `i64::MIN` and
    /// `i64::MAX`.
    ///
    /// For any Number on which `is_i64` returns true, `as_i64` is guaranteed to
    /// return the integer value.
    ///
    /// ```
    /// # fn main() -> Result<(), yaml_spanned::Error> {
    /// let big = i64::MAX as u64 + 10;
    /// let v: yaml_spanned::SpannedValue = yaml_spanned::from_str(r#"
    /// a: 64
    /// b: 9223372036854775817
    /// c: 256.0
    /// "#)?;
    ///
    /// assert!(v["a"].is_i64());
    ///
    /// // Greater than i64::MAX.
    /// assert!(!v["b"].is_i64());
    ///
    /// // Numbers with a decimal point are not considered integers.
    /// assert!(!v["c"].is_i64());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[allow(clippy::cast_sign_loss)]
    #[must_use] pub fn is_i64(&self) -> bool {
        match self.0 {
            N::PosInt(v) => i64::try_from(v).is_ok(),
            N::NegInt(_) => true,
            N::Float(_) => false,
        }
    }

    /// Returns true if the `Number` is an integer between zero and `u64::MAX`.
    ///
    /// For any Number on which `is_u64` returns true, `as_u64` is guaranteed to
    /// return the integer value.
    ///
    /// ```
    /// # fn main() -> Result<(), yaml_spanned::Error> {
    /// let v: yaml_spanned::SpannedValue = yaml_spanned::from_str(r#"
    /// a: 64
    /// b: -64
    /// c: 256.0
    /// "#)?;
    ///
    /// assert!(v["a"].is_u64());
    ///
    /// // Negative integer.
    /// assert!(!v["b"].is_u64());
    ///
    /// // Numbers with a decimal point are not considered integers.
    /// assert!(!v["c"].is_u64());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use] pub fn is_u64(&self) -> bool {
        match self.0 {
            N::PosInt(_) => true,
            N::NegInt(_) | N::Float(_) => false,
        }
    }

    /// Returns true if the `Number` can be represented by f64.
    ///
    /// For any Number on which `is_f64` returns true, `as_f64` is guaranteed to
    /// return the floating point value.
    ///
    /// Currently this function returns true if and only if both `is_i64` and
    /// `is_u64` return false but this is not a guarantee in the future.
    ///
    /// ```
    /// # fn main() -> Result<(), yaml_spanned::Error> {
    /// let v: yaml_spanned::SpannedValue = yaml_spanned::from_str(r#"
    /// a: 256.0
    /// b: 64
    /// c: -64
    /// "#)?;
    ///
    /// assert!(v["a"].is_f64());
    ///
    /// // Integers.
    /// assert!(!v["b"].is_f64());
    /// assert!(!v["c"].is_f64());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use] pub fn is_f64(&self) -> bool {
        match self.0 {
            N::Float(_) => true,
            N::PosInt(_) | N::NegInt(_) => false,
        }
    }

    /// If the `Number` is an integer, represent it as i64 if possible. Returns
    /// None otherwise.
    ///
    /// ```
    /// # fn main() -> Result<(), yaml_spanned::Error> {
    /// let big = i64::MAX as u64 + 10;
    /// let v: yaml_spanned::SpannedValue = yaml_spanned::from_str(r#"
    /// a: 64
    /// b: 9223372036854775817
    /// c: 256.0
    /// "#)?;
    ///
    /// assert_eq!(v["a"].as_i64(), Some(64));
    /// assert_eq!(v["b"].as_i64(), None);
    /// assert_eq!(v["c"].as_i64(), None);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use] pub fn as_i64(&self) -> Option<i64> {
        match self.0 {
            N::PosInt(n) => {
                if i64::try_from(n).is_ok() {
                    Some(n as i64)
                } else {
                    None
                }
            }
            N::NegInt(n) => Some(n),
            N::Float(_) => None,
        }
    }

    #[inline]
    pub fn as_i64_mut(&mut self) -> Option<&mut i64> {
        match self.0 {
            N::NegInt(ref mut n) => Some(n),
            N::PosInt(_) | N::Float(_) => None,
        }
    }

    /// If the `Number` is an integer, represent it as u64 if possible. Returns
    /// None otherwise.
    ///
    /// ```
    /// # fn main() -> Result<(), yaml_spanned::Error> {
    /// let v: yaml_spanned::SpannedValue = yaml_spanned::from_str(r#"
    /// a: 64
    /// b: -64
    /// c: 256.0
    /// "#)?;
    ///
    /// assert_eq!(v["a"].as_u64(), Some(64));
    /// assert_eq!(v["b"].as_u64(), None);
    /// assert_eq!(v["c"].as_u64(), None);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use] pub fn as_u64(&self) -> Option<u64> {
        match self.0 {
            N::PosInt(n) => Some(n),
            N::NegInt(_) | N::Float(_) => None,
        }
    }

    #[inline]
    pub fn as_u64_mut(&mut self) -> Option<&mut u64> {
        match self.0 {
            N::PosInt(ref mut n) => Some(n),
            N::NegInt(_) | N::Float(_) => None,
        }
    }

    /// Represents the number as f64 if possible. Returns None otherwise.
    ///
    /// ```
    /// # fn main() -> Result<(), yaml_spanned::Error> {
    /// let v: yaml_spanned::SpannedValue = yaml_spanned::from_str(r#"
    /// a: 256.0
    /// b: 64
    /// c: -64
    /// "#)?;
    ///
    /// assert_eq!(v["a"].as_f64(), Some(256.0));
    /// assert_eq!(v["b"].as_f64(), Some(64.0));
    /// assert_eq!(v["c"].as_f64(), Some(-64.0));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ```
    /// # fn main() -> Result<(), yaml_spanned::Error> {
    /// let v: yaml_spanned::SpannedValue = yaml_spanned::from_str(".inf")?;
    /// assert_eq!(v.as_f64(), Some(f64::INFINITY));
    ///
    /// let v: yaml_spanned::SpannedValue = yaml_spanned::from_str("-.inf")?;
    /// assert_eq!(v.as_f64(), Some(f64::NEG_INFINITY));
    ///
    /// let v: yaml_spanned::SpannedValue = yaml_spanned::from_str(".nan")?;
    /// assert!(v.as_f64().unwrap().is_nan());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use] pub fn as_f64(&self) -> Option<f64> {
        match self.0 {
            N::PosInt(n) => Some(n as f64),
            N::NegInt(n) => Some(n as f64),
            N::Float(n) => Some(n),
        }
    }

    #[inline]
    pub fn as_f64_mut(&mut self) -> Option<&mut f64> {
        match self.0 {
            N::PosInt(_) | N::NegInt(_) => None,
            N::Float(ref mut n) => Some(n),
        }
    }

    /// Returns true if this value is NaN and false otherwise.
    ///
    /// ```
    /// # use yaml_spanned::Number;
    /// #
    /// assert!(!Number::from(256.0).is_nan());
    ///
    /// assert!(Number::from(f64::NAN).is_nan());
    ///
    /// assert!(!Number::from(f64::INFINITY).is_nan());
    ///
    /// assert!(!Number::from(f64::NEG_INFINITY).is_nan());
    ///
    /// assert!(!Number::from(1).is_nan());
    /// ```
    #[inline]
    #[must_use] pub fn is_nan(&self) -> bool {
        match self.0 {
            N::PosInt(_) | N::NegInt(_) => false,
            N::Float(f) => f.is_nan(),
        }
    }

    /// Returns true if this value is positive infinity or negative infinity and
    /// false otherwise.
    ///
    /// ```
    /// # use yaml_spanned::Number;
    /// #
    /// assert!(!Number::from(256.0).is_infinite());
    ///
    /// assert!(!Number::from(f64::NAN).is_infinite());
    ///
    /// assert!(Number::from(f64::INFINITY).is_infinite());
    ///
    /// assert!(Number::from(f64::NEG_INFINITY).is_infinite());
    ///
    /// assert!(!Number::from(1).is_infinite());
    /// ```
    #[inline]
    #[must_use] pub fn is_infinite(&self) -> bool {
        match self.0 {
            N::PosInt(_) | N::NegInt(_) => false,
            N::Float(f) => f.is_infinite(),
        }
    }

    /// Returns true if this number is neither infinite nor NaN.
    ///
    /// ```
    /// # use yaml_spanned::Number;
    /// #
    /// assert!(Number::from(256.0).is_finite());
    ///
    /// assert!(!Number::from(f64::NAN).is_finite());
    ///
    /// assert!(!Number::from(f64::INFINITY).is_finite());
    ///
    /// assert!(!Number::from(f64::NEG_INFINITY).is_finite());
    ///
    /// assert!(Number::from(1).is_finite());
    /// ```
    #[inline]
    #[must_use] pub fn is_finite(&self) -> bool {
        match self.0 {
            N::PosInt(_) | N::NegInt(_) => true,
            N::Float(f) => f.is_finite(),
        }
    }
}

impl std::fmt::Debug for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            N::PosInt(n) => write!(f, "UnsignedInt({n})"),
            N::NegInt(n) => write!(f, "SignedInt({n})"),
            N::Float(n) => write!(f, "Float({n})"),
        }
    }
}

impl std::fmt::Display for Number {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.0 {
            N::PosInt(i) => formatter.write_str(itoa::Buffer::new().format(i)),
            N::NegInt(i) => formatter.write_str(itoa::Buffer::new().format(i)),
            N::Float(f) if f.is_nan() => formatter.write_str(".nan"),
            N::Float(f) if f.is_infinite() => {
                if f.is_sign_negative() {
                    formatter.write_str("-.inf")
                } else {
                    formatter.write_str(".inf")
                }
            }
            N::Float(f) => formatter.write_str(ryu::Buffer::new().format_finite(f)),
        }
    }
}

#[inline]
fn parse_unsigned_int<T>(
    scalar: &str,
    from_str_radix: fn(&str, radix: u32) -> Result<T, std::num::ParseIntError>,
    // ) -> Result<Option<T>, std::num::ParseIntError> {
) -> Option<T> {
    let unpositive = scalar.strip_prefix('+').unwrap_or(scalar);
    if let Some(rest) = unpositive.strip_prefix("0x") {
        if rest.starts_with(['+', '-']) {
            return None;
        }
        // let int = from_str_radix(rest, 16)?;
        // return Ok(Some(int));
        if let Ok(int) = from_str_radix(rest, 16) {
            return Some(int);
        }
    }
    if let Some(rest) = unpositive.strip_prefix("0o") {
        if rest.starts_with(['+', '-']) {
            return None;
        }
        // let int = from_str_radix(rest, 8)?;
        // return Some(int);
        if let Ok(int) = from_str_radix(rest, 8) {
            return Some(int);
        }
    }
    if let Some(rest) = unpositive.strip_prefix("0b") {
        if rest.starts_with(['+', '-']) {
            return None;
        }
        // let int = from_str_radix(rest, 2)?;
        // return Some(int);
        if let Ok(int) = from_str_radix(rest, 2) {
            return Some(int);
        }
    }
    if unpositive.starts_with(['+', '-']) {
        return None;
    }
    if digits_but_not_number(scalar) {
        return None;
    }
    from_str_radix(unpositive, 10).ok()
    // let int = from_str_radix(unpositive, 10)?;
    // return Ok(Some(int));
}

// #[inline]
// fn parse_signed_int<T>(
//     scalar: &str,
//     from_str_radix: fn(&str, radix: u32) -> Result<T, std::num::ParseIntError>,
// ) -> Option<T> {
//     let unpositive = if let Some(unpositive) = scalar.strip_prefix('+') {
//         if unpositive.starts_with(['+', '-']) {
//             return None;
//         }
//         unpositive
//     } else {
//         scalar
//     };
//     if let Some(rest) = unpositive.strip_prefix("0x") {
//         if rest.starts_with(['+', '-']) {
//             return None;
//         }
//         // let int = from_str_radix(rest, 16)?;
//         // return Ok(Some(int));
//         if let Ok(int) = from_str_radix(rest, 16) {
//             return Some(int);
//         }
//     }
//     if let Some(rest) = scalar.strip_prefix("-0x") {
//         let negative = format!("-{}", rest);
//         // let int = from_str_radix(&negative, 16)?;
//         // return Ok(Some(int));
//         if let Ok(int) = from_str_radix(&negative, 16) {
//             return Some(int);
//         }
//     }
//     if let Some(rest) = unpositive.strip_prefix("0o") {
//         if rest.starts_with(['+', '-']) {
//             return None;
//         }
//         // let int = from_str_radix(rest, 8)?;
//         // return Ok(Some(int));
//         if let Ok(int) = from_str_radix(rest, 8) {
//             return Some(int);
//         }
//     }
//     if let Some(rest) = scalar.strip_prefix("-0o") {
//         let negative = format!("-{}", rest);
//         // let int = from_str_radix(&negative, 8)?;
//         // return Ok(Some(int));
//         if let Ok(int) = from_str_radix(&negative, 8) {
//             return Some(int);
//         }
//     }
//     if let Some(rest) = unpositive.strip_prefix("0b") {
//         if rest.starts_with(['+', '-']) {
//             return None;
//         }
//         // let int = from_str_radix(rest, 2)?;
//         // return Ok(Some(int));
//         if let Ok(int) = from_str_radix(rest, 2) {
//             return Some(int);
//         }
//     }
//     if let Some(rest) = scalar.strip_prefix("-0b") {
//         let negative = format!("-{}", rest);
//         // let int = from_str_radix(&negative, 2)?;
//         // return Ok(Some(int));
//         if let Ok(int) = from_str_radix(&negative, 2) {
//             return Some(int);
//         }
//     }
//     if digits_but_not_number(scalar) {
//         return None;
//     }
//     from_str_radix(unpositive, 10).ok()
//     // let int = from_str_radix(unpositive, 10)?;
//     // return Ok(Some(int));
// }

fn parse_negative_int<T>(
    scalar: &str,
    from_str_radix: fn(&str, radix: u32) -> Result<T, std::num::ParseIntError>,
    // ) -> Result<Option<T>, std::num::ParseIntError> {
) -> Option<T> {
    if let Some(rest) = scalar.strip_prefix("-0x") {
        let negative = format!("-{rest}");
        // let int = from_str_radix(&negative, 16)?;
        // return Ok(Some(int));
        if let Ok(int) = from_str_radix(&negative, 16) {
            return Some(int);
        }
    }
    if let Some(rest) = scalar.strip_prefix("-0o") {
        let negative = format!("-{rest}");
        // let int = from_str_radix(&negative, 8)?;
        // return Ok(Some(int));
        if let Ok(int) = from_str_radix(&negative, 8) {
            return Some(int);
        }
    }
    if let Some(rest) = scalar.strip_prefix("-0b") {
        let negative = format!("-{rest}");
        // let int = from_str_radix(&negative, 2)?;
        // return Ok(Some(int));
        if let Ok(int) = from_str_radix(&negative, 2) {
            return Some(int);
        }
    }
    if digits_but_not_number(scalar) {
        return None;
    }
    from_str_radix(scalar, 10).ok()
    // let int = from_str_radix(scalar, 10)?;
    // Ok(Some(int))
}

// pub(crate) fn parse_f64(scalar: &str) -> Result<Option<f64>, std::num::ParseFloatError> {
pub(crate) fn parse_f64(scalar: &str) -> Option<f64> {
    let unpositive = if let Some(unpositive) = scalar.strip_prefix('+') {
        if unpositive.starts_with(['+', '-']) {
            return None;
        }
        unpositive
    } else {
        scalar
    };
    if let ".inf" | ".Inf" | ".INF" = unpositive {
        return Some(f64::INFINITY);
    }
    if let "-.inf" | "-.Inf" | "-.INF" = scalar {
        return Some(f64::NEG_INFINITY);
    }
    if let ".nan" | ".NaN" | ".NAN" = scalar {
        return Some(f64::NAN.copysign(1.0));
    }
    // let float = unpositive.parse::<f64>()?;
    // if float.is_finite() {
    //     Some(float)
    // } else {
    //     Ok(None)
    // }
    if let Ok(float) = unpositive.parse::<f64>() {
        if float.is_finite() {
            return Some(float);
        }
    }
    None
}

#[inline]
pub(crate) fn digits_but_not_number(scalar: &str) -> bool {
    // Leading zero(s) followed by numeric characters is a string according to
    // the YAML 1.2 spec. https://yaml.org/spec/1.2/spec.html#id2761292
    let scalar = scalar.strip_prefix(['-', '+']).unwrap_or(scalar);
    scalar.len() > 1 && scalar.starts_with('0') && scalar[1..].bytes().all(|b| b.is_ascii_digit())
}

#[inline]
// pub fn parse_number(value: &str) -> Result<Option<Number>, InvalidNumberError> {
pub fn parse_number(value: &str) -> Option<Number> {
    // dbg!(
    //     &value,
    //     parse_unsigned_int(value, u64::from_str_radix),
    //     parse_negative_int(value, i64::from_str_radix),
    //     parse_f64(value)
    // );
    // if let Some(unsigned) = parse_unsigned_int(value, u64::from_str_radix)? {
    //     return Ok(Some(N::PosInt(unsigned).into()));
    // }
    // if let Some(int) = parse_negative_int(value, i64::from_str_radix)? {
    //     return Ok(Some(N::NegInt(int).into()));
    // }
    // if let Some(float) = parse_f64(value)? {
    //     return Ok(Some(N::Float(float).into()));
    // }
    // Ok(None)

    if let Some(unsigned) = parse_unsigned_int(value, u64::from_str_radix) {
        return Some(unsigned.into());
    }
    if let Some(int) = parse_negative_int(value, i64::from_str_radix) {
        return Some(int.into());
    }
    if !digits_but_not_number(value) {
        if let Some(float) = parse_f64(value) {
            return Some(float.into());
        }
    }
    None
}

impl std::str::FromStr for Number {
    type Err = InvalidNumberError;

    fn from_str(repr: &str) -> Result<Self, Self::Err> {
        parse_number(repr).ok_or_else(|| InvalidNumberError::UnknownFormat(repr.to_string()))
        // parse_number(repr)?.ok_or_else(|| InvalidNumberError::UnknownFormat(repr.to_string()))
        //     .ok_or_else(|| crate::error::InvalidNumberError {
        // parse_number(repr).ok_or_else(|| crate::error::InvalidNumberError {
        //     value: repr.to_string(),
        // })
        // if let Ok(result) = de::visit_int(NumberVisitor, repr) {
        //     return result;
        // }
        // if !digits_but_not_number(repr) {
        //     if let Some(float) = parse_f64(repr) {
        //         return Ok(float.into());
        //     }
        // }
        // Err(error::new(ErrorImpl::FailedToParseNumber))
    }
}

impl PartialEq for N {
    fn eq(&self, other: &N) -> bool {
        match (*self, *other) {
            (N::PosInt(a), N::PosInt(b)) => a == b,
            (N::NegInt(a), N::NegInt(b)) => a == b,
            (N::Float(a), N::Float(b)) => {
                if a.is_nan() && b.is_nan() {
                    // YAML only has one NaN;
                    // the bit representation isn't preserved
                    true
                } else {
                    a == b
                }
            }
            _ => false,
        }
    }
}

impl PartialOrd for N {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (*self, *other) {
            (N::Float(a), N::Float(b)) => {
                if a.is_nan() && b.is_nan() {
                    // YAML only has one NaN
                    Some(Ordering::Equal)
                } else {
                    a.partial_cmp(&b)
                }
            }
            _ => Some(self.total_cmp(other)),
        }
    }
}

impl N {
    fn total_cmp(&self, other: &Self) -> Ordering {
        match (*self, *other) {
            (N::PosInt(a), N::PosInt(b)) => a.cmp(&b),
            (N::NegInt(a), N::NegInt(b)) => a.cmp(&b),
            // negint is always less than zero
            (N::NegInt(_), N::PosInt(_)) => Ordering::Less,
            (N::PosInt(_), N::NegInt(_)) => Ordering::Greater,
            (N::Float(a), N::Float(b)) => a.partial_cmp(&b).unwrap_or_else(|| {
                // arbitrarily sort the NaN last
                if !a.is_nan() {
                    Ordering::Less
                } else if !b.is_nan() {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            }),
            // arbitrarily sort integers below floats
            // FIXME: maybe something more sensible?
            (_, N::Float(_)) => Ordering::Less,
            (N::Float(_), _) => Ordering::Greater,
        }
    }
}

impl Number {
    pub(crate) fn total_cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[cfg(feature = "serde")]
pub mod serde {
    use super::{N, Number};

    pub(crate) fn unexpected(number: &Number) -> serde::de::Unexpected {
        match number.0 {
            N::PosInt(u) => serde::de::Unexpected::Unsigned(u),
            N::NegInt(i) => serde::de::Unexpected::Signed(i),
            N::Float(f) => serde::de::Unexpected::Float(f),
        }
    }

    impl serde::Serialize for Number {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match self.0 {
                N::PosInt(i) => serializer.serialize_u64(i),
                N::NegInt(i) => serializer.serialize_i64(i),
                N::Float(f) => serializer.serialize_f64(f),
            }
        }
    }

    struct NumberVisitor;

    impl serde::de::Visitor<'_> for NumberVisitor {
        type Value = Number;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a number")
        }

        #[inline]
        fn visit_i64<E>(self, value: i64) -> Result<Number, E> {
            Ok(value.into())
        }

        #[inline]
        fn visit_u64<E>(self, value: u64) -> Result<Number, E> {
            Ok(value.into())
        }

        #[inline]
        fn visit_f64<E>(self, value: f64) -> Result<Number, E> {
            Ok(value.into())
        }
    }

    impl<'de> serde::Deserialize<'de> for Number {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Number, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(NumberVisitor)
        }
    }

    impl<'de> serde::Deserializer<'de> for Number {
        type Error = crate::error::SerdeError;

        #[inline]
        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            match self.0 {
                N::PosInt(i) => visitor.visit_u64(i),
                N::NegInt(i) => visitor.visit_i64(i),
                N::Float(f) => visitor.visit_f64(f),
            }
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }

    impl<'de> serde::Deserializer<'de> for &Number {
        type Error = crate::error::SerdeError;

        #[inline]
        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            match self.0 {
                N::PosInt(i) => visitor.visit_u64(i),
                N::NegInt(i) => visitor.visit_i64(i),
                N::Float(f) => visitor.visit_f64(f),
            }
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
}

macro_rules! from_signed {
    ($($signed_ty:ident)*) => {
        $(
            impl From<$signed_ty> for Number {
                #[inline]
                #[allow(clippy::cast_sign_loss)]
                fn from(i: $signed_ty) -> Self {
                    if i < 0 {
                        Number(N::NegInt(i as i64))
                    } else {
                        Number(N::PosInt(i as u64))
                    }
                }
            }
        )*
    };
}

macro_rules! from_unsigned {
    ($($unsigned_ty:ident)*) => {
        $(
            impl From<$unsigned_ty> for Number {
                #[inline]
                fn from(u: $unsigned_ty) -> Self {
                    Number(N::PosInt(u as u64))
                }
            }
        )*
    };
}

from_signed!(i8 i16 i32 i64 isize);
from_unsigned!(u8 u16 u32 u64 usize);

impl From<f32> for Number {
    fn from(f: f32) -> Self {
        Number::from(f64::from(f))
    }
}

impl From<f64> for Number {
    fn from(mut f: f64) -> Self {
        if f.is_nan() {
            // Destroy NaN sign, signaling, and payload. YAML only has one NaN.
            f = f64::NAN.copysign(1.0);
        }
        Number(N::Float(f))
    }
}

// This is fine, because we don't _really_ implement hash for floats
// all other hash functions should work as expected
#[allow(clippy::derived_hash_with_manual_eq)]
impl std::hash::Hash for Number {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.0 {
            N::Float(_) => {
                // you should feel bad for using f64 as a map key
                3.hash(state);
            }
            N::PosInt(u) => u.hash(state),
            N::NegInt(i) => i.hash(state),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Mapping, Number, Value};
    use color_eyre::eyre;
    use indoc::indoc;
    use similar_asserts::assert_eq as sim_assert_eq;

    #[test]
    fn test_is_i64() -> eyre::Result<()> {
        crate::tests::init();
        // let big = i64::MAX as u64 + 10;
        let v: crate::SpannedValue = crate::from_str(indoc! {r"
            a: 64
            b: 9223372036854775817
            c: 256.0
        "})?;
        sim_assert_eq!(v["a"].as_i64(), Some(64), "a is an integer");
        sim_assert_eq!(v["b"].as_i64(), None, "b is greater than i64");
        sim_assert_eq!(
            v["c"].as_i64(),
            None,
            "numbers with a decimal point are not considered integers"
        );
        Ok(())
    }

    #[test]
    fn test_parse_number() -> eyre::Result<()> {
        crate::tests::init();
        let n = "111".parse::<Number>()?;
        sim_assert_eq!(n, Number::from(111));

        let n = "-111".parse::<Number>()?;
        sim_assert_eq!(n, Number::from(-111));

        let n = "-1.1".parse::<Number>()?;
        sim_assert_eq!(n, Number::from(-1.1));

        let n = ".nan".parse::<Number>()?;
        sim_assert_eq!(n, Number::from(f64::NAN));
        assert!(n.as_f64().unwrap().is_sign_positive());

        let n = ".inf".parse::<Number>()?;
        sim_assert_eq!(n, Number::from(f64::INFINITY));

        let n = "-.inf".parse::<Number>()?;
        sim_assert_eq!(n, Number::from(f64::NEG_INFINITY));

        let err = "null".parse::<Number>().unwrap_err();
        sim_assert_eq!(err.to_string(), r#"unknown number format "null""#);

        let err = " 1 ".parse::<Number>().unwrap_err();
        sim_assert_eq!(err.to_string(), r#"unknown number format " 1 ""#);
        Ok(())
    }

    #[test]
    fn test_numbers() -> eyre::Result<()> {
        crate::tests::init();
        let cases = [
            ("0xF0", "240"),
            ("+0xF0", "240"),
            ("-0xF0", "-240"),
            ("0o70", "56"),
            ("+0o70", "56"),
            ("-0o70", "-56"),
            ("0b10", "2"),
            ("+0b10", "2"),
            ("-0b10", "-2"),
            ("127", "127"),
            ("+127", "127"),
            ("-127", "-127"),
            (".inf", ".inf"),
            (".Inf", ".inf"),
            (".INF", ".inf"),
            ("-.inf", "-.inf"),
            ("-.Inf", "-.inf"),
            ("-.INF", "-.inf"),
            (".nan", ".nan"),
            (".NaN", ".nan"),
            (".NAN", ".nan"),
            ("0.1", "0.1"),
        ];
        for &(yaml, expected) in &cases {
            let value = crate::from_str(yaml)?.into_inner();
            match value {
                Value::Number(number) => sim_assert_eq!(number.to_string(), expected),
                _ => eyre::bail!("expected number. input={:?}, result={:?}", yaml, value),
            }
        }

        // NOT numbers.
        let cases = [
            "0127", "+0127", "-0127", "++.inf", "+-.inf", "++1", "+-1", "-+1", "--1", "0x+1",
            "0x-1", "-0x+1", "-0x-1", "++0x1", "+-0x1", "-+0x1", "--0x1",
        ];
        for yaml in &cases {
            let value = crate::from_str(yaml)?.into_inner();
            match value {
                Value::String(string) => sim_assert_eq!(string, *yaml),
                _ => eyre::bail!("expected string. input={:?}, result={:?}", yaml, value),
            }
        }
        Ok(())
    }

    #[test]
    fn test_number_alias_as_string() -> eyre::Result<()> {
        crate::tests::init();
        let yaml = indoc! {"
            version: &a 1.10
            value: *a
        "};
        let value = crate::from_str(yaml)?;
        sim_assert_eq!(
            value.clone().cleared_spans().into_inner(),
            Value::from(Mapping::from_iter([
                ("version".into(), 1.10.into()),
                ("value".into(), 1.10.into()),
            ]))
        );

        #[cfg(feature = "serde")]
        {
            #[derive(serde::Deserialize, PartialEq, Debug)]
            struct Num {
                version: String,
                value: String,
            }
            let _expected = Num {
                version: "1.10".to_string(),
                value: "1.10".to_string(),
            };
            // TODO: once the value uses number, this will fail, which is why serde_yaml does not
            // deserialize to a value first.
            // sim_assert_eq!(crate::from_value::<Num>(value)?, expected);
        }
        Ok(())
    }

    #[ignore = "cannot deserialize u128 from value"]
    #[test]
    fn test_u128_big() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            18446744073709551616
        "};

        let octal = indoc! {"
            0o2000000000000000000000
        "};

        #[cfg(feature = "serde")]
        {
            let expected: u128 = u128::from(u64::MAX) + 1;

            sim_assert_eq!(
                crate::from_value::<u128>(crate::from_str(yaml)?.as_ref())?,
                expected
            );
            sim_assert_eq!(
                crate::from_value::<u128>(crate::from_str(octal)?.as_ref())?,
                expected
            );
        }
        Ok(())
    }

    #[ignore = "cannot deserialize i128 from value"]
    #[test]
    fn test_i128_big() -> eyre::Result<()> {
        crate::tests::init();
        let yaml = indoc! {"
            -9223372036854775809
        "};
        let octal = indoc! {"
            -0o1000000000000000000001
        "};

        #[cfg(feature = "serde")]
        {
            let expected: i128 = i128::from(i64::MIN) - 1;
            sim_assert_eq!(
                crate::from_value::<i128>(crate::from_str(yaml)?.as_ref())?,
                expected
            );
            sim_assert_eq!(
                crate::from_value::<i128>(crate::from_str(octal)?.as_ref())?,
                expected
            );
        }
        Ok(())
    }

    #[test]
    fn test_number_as_string() -> eyre::Result<()> {
        crate::tests::init();
        let yaml = indoc! {"
            # Cannot be represented as u128
            value: 340282366920938463463374607431768211457
        "};

        // test assumption with serde_yaml
        sim_assert_eq!(
            serde_yaml::from_str::<serde_yaml::Value>(yaml)?,
            serde_yaml::Value::from(serde_yaml::Mapping::from_iter([(
                "value".into(),
                340282366920938463463374607431768211457f64.into()
            )]))
        );

        let value = crate::from_str(yaml)?;
        sim_assert_eq!(
            value.clone().cleared_spans().into_inner(),
            Value::from(Mapping::from_iter([(
                "value".into(),
                340282366920938463463374607431768211457f64.into()
            ),]))
        );

        #[cfg(feature = "serde")]
        {
            #[derive(serde::Deserialize, PartialEq, Debug)]
            struct Num {
                value: String,
            }
            let _expected = Num {
                value: "340282366920938463463374607431768211457".to_owned(),
            };
            // TODO: cannot deserialize float value into string
            // sim_assert_eq!(crate::from_value::<Num>(value)?, expected);
        }
        Ok(())
    }

    #[test]
    fn test_nan() -> eyre::Result<()> {
        crate::tests::init();

        let pos_nan = crate::from_str(".nan")?;
        assert!(pos_nan.is_f64());
        sim_assert_eq!(pos_nan, pos_nan);

        let neg_fake_nan = crate::from_str("-.nan")?;
        assert!(neg_fake_nan.is_string());

        let significand_mask = 0xF_FFFF_FFFF_FFFF;
        let bits = (f64::NAN.copysign(1.0).to_bits() ^ significand_mask) | 1;
        let different_pos_nan = Value::Number(Number::from(f64::from_bits(bits)));
        sim_assert_eq!(pos_nan, different_pos_nan);

        Ok(())
    }

    #[test]
    fn test_digits() -> eyre::Result<()> {
        crate::tests::init();
        let num_string = crate::from_str("01")?;
        assert!(num_string.is_string());
        Ok(())
    }
}
