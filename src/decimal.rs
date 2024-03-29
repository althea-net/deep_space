//! Decimal type with equivalent semantics to the [Cosmos `sdk.Dec`][1] type.
//! Imported from github.com/cosmos/cosmos-rust
//!
//! [1]: https://pkg.go.dev/github.com/cosmos/cosmos-sdk/types#Dec

use rust_decimal::Error as DecimalLibraryError;
use std::{
    convert::{TryFrom, TryInto},
    fmt::{self, Debug, Display},
    str::FromStr,
};

#[derive(Debug)]
pub enum DecimalError {
    ExcessivePrecision,
    InvalidPrecision,
    DecimalError(DecimalLibraryError),
}

impl fmt::Display for DecimalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecimalError::ExcessivePrecision => {
                write!(f, "Decimal exceeds maximum fractional digits")
            }
            DecimalError::InvalidPrecision => {
                write!(f, "Decimal is using an invalid precision must be 0 or 18")
            }
            DecimalError::DecimalError(v) => {
                write!(f, "{v:?}")
            }
        }
    }
}

impl std::error::Error for DecimalError {}

impl From<DecimalLibraryError> for DecimalError {
    fn from(error: DecimalLibraryError) -> Self {
        DecimalError::DecimalError(error)
    }
}

/// Number of decimal places required by an `sdk.Dec`
/// See: <https://github.com/cosmos/cosmos-sdk/blob/018915b/types/decimal.go#L23>
pub const PRECISION: u32 = 18;

/// Maximum value of the decimal part of an `sdk.Dec`
pub const FRACTIONAL_DIGITS_MAX: u64 = 9_999_999_999_999_999_999;

/// Decimal type which follows Cosmos [Cosmos `sdk.Dec`][1] conventions.
///
/// [1]: https://pkg.go.dev/github.com/cosmos/cosmos-sdk/types#Dec
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct Decimal(rust_decimal::Decimal);

impl Decimal {
    /// Create a new [`Decimal`] with the given whole number and decimal
    /// parts. The decimal part assumes 18 digits of precision e.g. a
    /// decimal with `(1, 1)` is `1.000000000000000001`.
    ///
    /// 18 digits required by the Cosmos SDK. See:
    /// See: <https://github.com/cosmos/cosmos-sdk/blob/26d6e49/types/decimal.go#L23>
    pub fn new(integral_digits: i64, fractional_digits: u64) -> Result<Self, DecimalError> {
        if fractional_digits > FRACTIONAL_DIGITS_MAX {
            return Err(DecimalError::ExcessivePrecision);
        }

        let integral_digits: rust_decimal::Decimal = integral_digits.into();
        let fractional_digits: rust_decimal::Decimal = fractional_digits.into();
        let precision_exp: rust_decimal::Decimal = 10u64.pow(PRECISION).into();

        let mut combined_decimal = (integral_digits * precision_exp) + fractional_digits;
        combined_decimal.set_scale(PRECISION)?;
        Ok(Decimal(combined_decimal))
    }
}

impl Debug for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Decimal {
    type Err = DecimalError;
    fn from_str(s: &str) -> Result<Self, DecimalError> {
        s.parse::<rust_decimal::Decimal>()?.try_into()
    }
}

impl TryFrom<rust_decimal::Decimal> for Decimal {
    type Error = DecimalError;
    fn try_from(mut decimal_value: rust_decimal::Decimal) -> Result<Self, DecimalError> {
        match decimal_value.scale() {
            0 => {
                let exp: rust_decimal::Decimal = 10u64.pow(PRECISION).into();
                decimal_value *= exp;
                decimal_value.set_scale(PRECISION)?;
            }
            PRECISION => (),
            _other => return Err(DecimalError::InvalidPrecision),
        }

        Ok(Decimal(decimal_value))
    }
}

macro_rules! impl_from_primitive_int_for_decimal {
    ($($int:ty),+) => {
        $(impl From<$int> for Decimal {
            fn from(num: $int) -> Decimal {
                #[allow(trivial_numeric_casts)]
                Decimal::new(num as i64, 0).unwrap()
            }
        })+
    };
}

impl_from_primitive_int_for_decimal!(i8, i16, i32, i64, isize);
impl_from_primitive_int_for_decimal!(u8, u16, u32, u64, usize);

#[cfg(test)]
mod tests {
    use super::Decimal;

    #[test]
    fn string_serialization_test() {
        let num = Decimal::from(-1i8);
        assert_eq!(num.to_string(), "-1.000000000000000000")
    }
}
