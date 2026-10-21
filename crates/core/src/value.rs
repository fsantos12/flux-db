//! Database values and conversions between Rust types and [`Value`].

use crate::error::Error;

/// A database value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    F32(f32),
    F64(f64),
    Decimal(rust_decimal::Decimal),
    Char(char),
    String(String),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
    Timestamp(chrono::NaiveDateTime),
    TimestampTz(chrono::DateTime<chrono::Utc>),
    Bytes(Vec<u8>),
    Uuid(uuid::Uuid),
    Json(serde_json::Value),
}

/// Converts a Rust type into a [`Value`].
pub trait IntoValue {
    fn into_value(self) -> Value;
}

/// Converts a [`Value`] back into a Rust type.
pub trait FromValue: Sized {
    fn from_value(value: Value) -> Result<Self, Error>;
}
