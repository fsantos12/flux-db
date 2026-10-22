//! Database values and conversions between Rust types and [`Value`].

use crate::error::{Error, Result};

/// Converts a Rust type into a [`Value`].
pub trait IntoValue {
    fn into_value(self) -> Value;
}

/// Converts a [`Value`] back into a Rust type.
pub trait FromValue: Sized {
    fn from_value(value: Value) -> Result<Self>;
}

/// Generates the [`Value`] enum, `type_name()` and the `IntoValue` / `FromValue`
/// impls. Each entry is `Variant(Type) = "name"`, optionally followed by
/// `widen [Variants...]`: smaller variants accepted losslessly via `From`.
macro_rules! define_value {
    ($( $variant:ident($ty:ty) = $name:literal $(widen [$($wide:ident),*])? ; )*) => {
        /// A database value.
        #[derive(Debug, Clone, PartialEq)]
        #[non_exhaustive]
        pub enum Value {
            /// SQL `NULL`.
            Null,
            $( $variant($ty), )*
        }

        impl Value {
            /// Returns a static name for this value's variant (for conversion errors).
            pub fn type_name(&self) -> &'static str {
                match self {
                    Self::Null => "null",
                    $( Self::$variant(_) => $name, )*
                }
            }
        }

        $(
            impl IntoValue for $ty {
                fn into_value(self) -> Value {
                    Value::$variant(self)
                }
            }

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
    Decimal(rust_decimal::Decimal) = "Decimal";
    Char(char) = "char";
    String(String) = "String";
    Date(chrono::NaiveDate) = "Date";
    Time(chrono::NaiveTime) = "Time";
    Timestamp(chrono::NaiveDateTime) = "Timestamp";
    TimestampTz(chrono::DateTime<chrono::Utc>) = "DateTime<Utc>";
    Bytes(Vec<u8>) = "Vec<u8>";
    Uuid(uuid::Uuid) = "Uuid";
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
            Value::Null => Ok(None),
            other => T::from_value(other).map(Some),
        }
    }
}
