//! Database values and conversions between Rust types and [`Value`].
//!
//! [`Value`] is the database-agnostic representation of a single cell.
//! [`IntoValue`] and [`FromValue`] convert between it and Rust types.
//! Only lossless numeric widening is accepted implicitly (e.g. `I8` into
//! `i64`, `F32` into `f64`); anything else returns [`Error::Conversion`].
//!
//! # Examples
//!
//! ```
//! use flux_db_core::types::value::{FromValue, IntoValue, Value};
//!
//! let value = 7_i8.into_value();
//! assert_eq!(value, Value::I8(7));
//! // Widening is lossless, so it is accepted.
//! assert_eq!(i64::from_value(value).unwrap(), 7);
//! // Narrowing could truncate, so it is rejected.
//! assert!(i8::from_value(Value::I64(7)).is_err());
//! ```

use crate::types::error::{Error, Result};

/// Converts a Rust type into a [`Value`].
pub trait IntoValue {
    /// Consumes `self` and returns the matching [`Value`]; `None` becomes [`Value::Null`].
    fn into_value(self) -> Value;
}

/// Converts a [`Value`] back into a Rust type.
pub trait FromValue: Sized {
    /// Converts `value` into `Self`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Conversion`] when the variant does not match `Self`
    /// and cannot be widened into it losslessly, including [`Value::Null`]
    /// into a non-`Option` type.
    fn from_value(value: Value) -> Result<Self>;
}

/// Generates the [`Value`] enum, `type_name()` and the `IntoValue` / `FromValue`
/// impls. Each entry is `Variant(Type) = "name"`, optionally followed by
/// `widen [Variants...]`: smaller variants accepted losslessly via `From`.
///
/// An entry may start with `#[cfg(...)]` attributes; they are applied to the
/// variant, its `type_name()` arm and both impls, so optional types live
/// behind Cargo features.
///
/// Keeping every type in one table guarantees the enum, its names and both
/// conversions never drift apart; add new types here, not by hand.
macro_rules! define_value {
    ($( $(#[$attr:meta])* $variant:ident($ty:ty) = $name:literal $(widen [$($wide:ident),*])? ; )*) => {
        /// A database value.
        #[derive(Debug, Clone, PartialEq)]
        #[non_exhaustive]
        pub enum Value {
            /// SQL `NULL`.
            Null,
            $(
                #[doc = concat!("A `", $name, "` value.")]
                $(#[$attr])*
                $variant($ty),
            )*
        }

        impl Value {
            /// Returns a static name for this value's variant, used in conversion errors.
            ///
            /// # Examples
            ///
            /// ```
            /// use flux_db_core::types::value::Value;
            ///
            /// assert_eq!(Value::Null.type_name(), "null");
            /// assert_eq!(Value::Bool(true).type_name(), "bool");
            /// ```
            pub fn type_name(&self) -> &'static str {
                match self {
                    Self::Null => "null",
                    $( $(#[$attr])* Self::$variant(_) => $name, )*
                }
            }
        }

        $(
            $(#[$attr])*
            impl IntoValue for $ty {
                fn into_value(self) -> Value {
                    Value::$variant(self)
                }
            }

            $(#[$attr])*
            impl FromValue for $ty {
                fn from_value(value: Value) -> Result<Self> {
                    match value {
                        Value::$variant(v) => Ok(v),
                        $($( Value::$wide(v) => Ok(<$ty>::from(v)), )*)?
                        other => Err(Error::conversion($name, other.type_name())),
                    }
                }
            }
        )*
    };
}

define_value! {
    Bool(bool) = "bool";
    I8(i8) = "i8";
    I16(i16) = "i16" widen [I8, U8];
    I32(i32) = "i32" widen [I8, I16, U8, U16];
    I64(i64) = "i64" widen [I8, I16, I32, U8, U16, U32];
    I128(i128) = "i128" widen [I8, I16, I32, I64, U8, U16, U32, U64];
    U8(u8) = "u8";
    U16(u16) = "u16" widen [U8];
    U32(u32) = "u32" widen [U8, U16];
    U64(u64) = "u64" widen [U8, U16, U32];
    U128(u128) = "u128" widen [U8, U16, U32, U64];
    F32(f32) = "f32";
    F64(f64) = "f64" widen [F32];
    #[cfg(feature = "decimal")]
    Decimal(rust_decimal::Decimal) = "Decimal";
    Char(char) = "char";
    String(String) = "String";
    #[cfg(feature = "chrono")]
    Date(chrono::NaiveDate) = "Date";
    #[cfg(feature = "chrono")]
    Time(chrono::NaiveTime) = "Time";
    #[cfg(feature = "chrono")]
    Timestamp(chrono::NaiveDateTime) = "Timestamp";
    #[cfg(feature = "chrono")]
    TimestampTz(chrono::DateTime<chrono::Utc>) = "DateTime<Utc>";
    Bytes(Vec<u8>) = "Vec<u8>";
    #[cfg(feature = "uuid")]
    Uuid(uuid::Uuid) = "Uuid";
    #[cfg(feature = "json")]
    Json(serde_json::Value) = "Json";
}

impl IntoValue for &str {
    fn into_value(self) -> Value {
        Value::String(self.to_owned())
    }
}

impl IntoValue for &[u8] {
    fn into_value(self) -> Value {
        Value::Bytes(self.to_owned())
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(self) -> Value {
        self.map_or(Value::Null, IntoValue::into_value)
    }
}

impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            // NULL is the only value that maps to `None`; everything else must convert.
            Value::Null => Ok(None),
            other => T::from_value(other).map(Some),
        }
    }
}
