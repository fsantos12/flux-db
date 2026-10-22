//! Database values and conversions between Rust types and [`Value`].

use crate::error::{Error, Result};

/// A database value.
macro_rules! define_value_enum {
    ($( $variant:ident , $ty:ty , $const_name:ident , $custom_str:expr ; )*) => {
        #[derive(Debug, Clone, PartialEq)]
        pub enum Value {
            $( $variant($ty), )*
        }

        impl Value {
            $(
                pub const $const_name: &'static str = $custom_str;
            )*
        }
    };
}

define_value_enum! {
    Null, (), NULL_TYPE_NAME, "null";
    Bool, bool, BOOL_TYPE_NAME, "bool";
    I8, i8, I8_TYPE_NAME, "i8";
    I16, i16, I16_TYPE_NAME, "i16";
    I32, i32, I32_TYPE_NAME, "i32";
    I64, i64, I64_TYPE_NAME, "i64";
    I128, i128, I128_TYPE_NAME, "i128";
    U8, u8, U8_TYPE_NAME, "u8";
    U16, u16, U16_TYPE_NAME, "u16";
    U32, u32, U32_TYPE_NAME, "u32";
    U64, u64, U64_TYPE_NAME, "u64";
    U128, u128, U128_TYPE_NAME, "u128";
    F32, f32, F32_TYPE_NAME, "f32";
    F64, f64, F64_TYPE_NAME, "f64";
    Decimal, rust_decimal::Decimal, DECIMAL_TYPE_NAME, "Decimal";
    Char, char, CHAR_TYPE_NAME, "char";
    String, String, STRING_TYPE_NAME, "String";
    Date, chrono::NaiveDate, DATE_TYPE_NAME, "Date";
    Time, chrono::NaiveTime, TIME_TYPE_NAME, "Time";
    Timestamp, chrono::NaiveDateTime, TIMESTAMP_TYPE_NAME, "Timestamp";
    TimestampTz, chrono::DateTime<chrono::Utc>, TIMESTAMP_TZ_TYPE_NAME, "DateTime<Utc>";
    Bytes, Vec<u8>, BYTES_TYPE_NAME, "Vec<u8>";
    Uuid, uuid::Uuid, UUID_TYPE_NAME, "Uuid";
    Json, serde_json::Value, JSON_TYPE_NAME, "Json";
}

impl Value {
    /// Returns a static name for this value's variant (for conversion errors).
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null(_) => Self::NULL_TYPE_NAME,
            Self::Bool(_) => Self::BOOL_TYPE_NAME,
            Self::I8(_) => Self::I8_TYPE_NAME,
            Self::I16(_) => Self::I16_TYPE_NAME,
            Self::I32(_) => Self::I32_TYPE_NAME,
            Self::I64(_) => Self::I64_TYPE_NAME,
            Self::I128(_) => Self::I128_TYPE_NAME,
            Self::U8(_) => Self::U8_TYPE_NAME,
            Self::U16(_) => Self::U16_TYPE_NAME,
            Self::U32(_) => Self::U32_TYPE_NAME,
            Self::U64(_) => Self::U64_TYPE_NAME,
            Self::U128(_) => Self::U128_TYPE_NAME,
            Self::F32(_) => Self::F32_TYPE_NAME,
            Self::F64(_) => Self::F64_TYPE_NAME,
            Self::Decimal(_) => Self::DECIMAL_TYPE_NAME,
            Self::Char(_) => Self::CHAR_TYPE_NAME,
            Self::String(_) => Self::STRING_TYPE_NAME,
            Self::Date(_) => Self::DATE_TYPE_NAME,
            Self::Time(_) => Self::TIME_TYPE_NAME,
            Self::Timestamp(_) => Self::TIMESTAMP_TYPE_NAME,
            Self::TimestampTz(_) => Self::TIMESTAMP_TZ_TYPE_NAME,
            Self::Bytes(_) => Self::BYTES_TYPE_NAME,
            Self::Uuid(_) => Self::UUID_TYPE_NAME,
            Self::Json(_) => Self::JSON_TYPE_NAME,
        }
    }
}

/// Converts a Rust type into a [`Value`].
pub trait IntoValue {
    fn into_value(self) -> Value;
}

/// Converts a [`Value`] back into a Rust type.
pub trait FromValue: Sized {
    fn from_value(value: Value) -> Result<Self>;
}

impl IntoValue for bool {
    fn into_value(self) -> Value {
        Value::Bool(self)
    }
}

impl FromValue for bool {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Bool(v) => Ok(v),
            other => Err(Error::conversion("bool", other.type_name())),
        }
    }
}

impl IntoValue for i8 {
    fn into_value(self) -> Value {
        Value::I8(self)
    }
}

impl FromValue for i8 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::I8(v) => Ok(v),
            other => Err(Error::conversion("i8", other.type_name())),
        }
    }
}

impl IntoValue for i16 {
    fn into_value(self) -> Value {
        Value::I16(self)
    }
}

impl FromValue for i16 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::I16(v) => Ok(v),
            other => Err(Error::conversion("i16", other.type_name())),
        }
    }
}

impl IntoValue for i32 {
    fn into_value(self) -> Value {
        Value::I32(self)
    }
}

impl FromValue for i32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::I32(v) => Ok(v),
            other => Err(Error::conversion("i32", other.type_name())),
        }
    }
}

impl IntoValue for i64 {
    fn into_value(self) -> Value {
        Value::I64(self)
    }
}

impl FromValue for i64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::I64(v) => Ok(v),
            other => Err(Error::conversion("i64", other.type_name())),
        }
    }
}

impl IntoValue for i128 {
    fn into_value(self) -> Value {
        Value::I128(self)
    }
}

impl FromValue for i128 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::I128(v) => Ok(v),
            other => Err(Error::conversion("i128", other.type_name())),
        }
    }
}

impl IntoValue for u8 {
    fn into_value(self) -> Value {
        Value::U8(self)
    }
}

impl FromValue for u8 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::U8(v) => Ok(v),
            other => Err(Error::conversion("u8", other.type_name())),
        }
    }
}

impl IntoValue for u16 {
    fn into_value(self) -> Value {
        Value::U16(self)
    }
}

impl FromValue for u16 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::U16(v) => Ok(v),
            other => Err(Error::conversion("u16", other.type_name())),
        }
    }
}

impl IntoValue for u32 {
    fn into_value(self) -> Value {
        Value::U32(self)
    }
}

impl FromValue for u32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::U32(v) => Ok(v),
            other => Err(Error::conversion("u32", other.type_name())),
        }
    }
}

impl IntoValue for u64 {
    fn into_value(self) -> Value {
        Value::U64(self)
    }
}

impl FromValue for u64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::U64(v) => Ok(v),
            other => Err(Error::conversion("u64", other.type_name())),
        }
    }
}

impl IntoValue for u128 {
    fn into_value(self) -> Value {
        Value::U128(self)
    }
}

impl FromValue for u128 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::U128(v) => Ok(v),
            other => Err(Error::conversion("u128", other.type_name())),
        }
    }
}

impl IntoValue for f32 {
    fn into_value(self) -> Value {
        Value::F32(self)
    }
}

impl FromValue for f32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::F32(v) => Ok(v),
            other => Err(Error::conversion("f32", other.type_name())),
        }
    }
}

impl IntoValue for f64 {
    fn into_value(self) -> Value {
        Value::F64(self)
    }
}

impl FromValue for f64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::F64(v) => Ok(v),
            other => Err(Error::conversion("f64", other.type_name())),
        }
    }
}

impl IntoValue for rust_decimal::Decimal {
    fn into_value(self) -> Value {
        Value::Decimal(self)
    }
}

impl FromValue for rust_decimal::Decimal {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Decimal(v) => Ok(v),
            other => Err(Error::conversion("Decimal", other.type_name())),
        }
    }
}

impl IntoValue for char {
    fn into_value(self) -> Value {
        Value::Char(self)
    }
}

impl FromValue for char {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Char(v) => Ok(v),
            other => Err(Error::conversion("char", other.type_name())),
        }
    }
}

impl IntoValue for String {
    fn into_value(self) -> Value {
        Value::String(self)
    }
}

impl FromValue for String {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::String(v) => Ok(v),
            other => Err(Error::conversion("String", other.type_name())),
        }
    }
}

impl IntoValue for &str {
    fn into_value(self) -> Value {
        Value::String(self.to_owned())
    }
}

impl IntoValue for chrono::NaiveDate {
    fn into_value(self) -> Value {
        Value::Date(self)
    }
}

impl FromValue for chrono::NaiveDate {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Date(v) => Ok(v),
            other => Err(Error::conversion("NaiveDate", other.type_name())),
        }
    }
}

impl IntoValue for chrono::NaiveTime {
    fn into_value(self) -> Value {
        Value::Time(self)
    }
}

impl FromValue for chrono::NaiveTime {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Time(v) => Ok(v),
            other => Err(Error::conversion("NaiveTime", other.type_name())),
        }
    }
}

impl IntoValue for chrono::NaiveDateTime {
    fn into_value(self) -> Value {
        Value::Timestamp(self)
    }
}

impl FromValue for chrono::NaiveDateTime {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Timestamp(v) => Ok(v),
            other => Err(Error::conversion("NaiveDateTime", other.type_name())),
        }
    }
}

impl IntoValue for chrono::DateTime<chrono::Utc> {
    fn into_value(self) -> Value {
        Value::TimestampTz(self)
    }
}

impl FromValue for chrono::DateTime<chrono::Utc> {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::TimestampTz(v) => Ok(v),
            other => Err(Error::conversion("DateTime<Utc>", other.type_name())),
        }
    }
}

impl IntoValue for Vec<u8> {
    fn into_value(self) -> Value {
        Value::Bytes(self)
    }
}

impl FromValue for Vec<u8> {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Bytes(v) => Ok(v),
            other => Err(Error::conversion("Vec<u8>", other.type_name())),
        }
    }
}

impl IntoValue for &[u8] {
    fn into_value(self) -> Value {
        Value::Bytes(self.to_owned())
    }
}

impl IntoValue for uuid::Uuid {
    fn into_value(self) -> Value {
        Value::Uuid(self)
    }
}

impl FromValue for uuid::Uuid {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Uuid(v) => Ok(v),
            other => Err(Error::conversion("Uuid", other.type_name())),
        }
    }
}

impl IntoValue for serde_json::Value {
    fn into_value(self) -> Value {
        Value::Json(self)
    }
}

impl FromValue for serde_json::Value {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Json(v) => Ok(v),
            other => Err(Error::conversion("Json", other.type_name())),
        }
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(self) -> Value {
        match self {
            Some(v) => v.into_value(),
            None => Value::Null(()),
        }
    }
}

impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Null(_) => Ok(None),
            other => T::from_value(other).map(Some),
        }
    }
}
