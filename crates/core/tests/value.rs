//! Unit-level tests for `Value`, `IntoValue`, `FromValue` and `Error`.

use std::error::Error as StdError;

use flux_db_core::types::error::Error;
use flux_db_core::types::value::{FromValue, IntoValue, Value};

/// Asserts that converting `value` into `T` fails with `Conversion`
/// carrying the given expected and received names.
fn assert_conversion<T: FromValue + std::fmt::Debug>(value: Value, expected: &str, received: &str) {
    match T::from_value(value) {
        Err(Error::Conversion {
            expected: e,
            received: r,
        }) => {
            assert_eq!(e, expected);
            assert_eq!(r, received);
        }
        other => panic!("expected Conversion, got {other:?}"),
    }
}

fn roundtrip<T: IntoValue + FromValue + PartialEq + std::fmt::Debug + Clone>(v: T) {
    assert_eq!(T::from_value(v.clone().into_value()).unwrap(), v);
}

#[test]
fn every_type_round_trips_including_extremes() {
    roundtrip(true);
    roundtrip(false);
    roundtrip(i8::MIN);
    roundtrip(i8::MAX);
    roundtrip(i16::MIN);
    roundtrip(i32::MIN);
    roundtrip(i64::MIN);
    roundtrip(i64::MAX);
    roundtrip(i128::MIN);
    roundtrip(u8::MAX);
    roundtrip(u16::MAX);
    roundtrip(u32::MAX);
    roundtrip(u64::MAX);
    roundtrip(u128::MAX);
    roundtrip(f32::MAX);
    roundtrip(f64::MIN_POSITIVE);
    roundtrip(f64::INFINITY);
    roundtrip(f64::NEG_INFINITY);
    roundtrip(rust_decimal::Decimal::MAX);
    roundtrip('\u{1F980}');
    roundtrip(String::new());
    roundtrip("emoji 🦀, combining e\u{301}, RTL עברית".to_owned());
    roundtrip(chrono::NaiveDate::MIN);
    roundtrip(chrono::NaiveDate::MAX);
    roundtrip(chrono::NaiveTime::from_hms_micro_opt(23, 59, 59, 999_999).unwrap());
    roundtrip(chrono::NaiveDateTime::MAX);
    roundtrip(chrono::DateTime::<chrono::Utc>::MIN_UTC);
    roundtrip(Vec::<u8>::new());
    roundtrip(vec![0u8, 255, 0]);
    roundtrip(uuid::Uuid::nil());
    roundtrip(uuid::Uuid::max());
    roundtrip(serde_json::json!({"a": [1, null, "x"]}));
    roundtrip(serde_json::Value::Null);
}

#[test]
fn text_with_nul_byte_is_kept_in_value() {
    // Postgres rejects NUL in text; that is the driver's job, the Value must not alter it.
    let v = "a\0b".into_value();
    assert_eq!(v, Value::String("a\0b".to_owned()));
}

#[test]
fn borrowed_str_and_bytes_become_owned_values() {
    assert_eq!("hi".into_value(), Value::String("hi".to_owned()));
    assert_eq!((&[1u8, 2][..]).into_value(), Value::Bytes(vec![1, 2]));
}

#[test]
fn nan_round_trips_but_is_not_equal_to_itself() {
    let back = f64::from_value(f64::NAN.into_value()).unwrap();
    assert!(back.is_nan());
    // Documents derived PartialEq: NaN values never compare equal.
    assert_ne!(Value::F64(f64::NAN), Value::F64(f64::NAN));
}

#[test]
fn negative_zero_keeps_its_sign() {
    let back = f64::from_value((-0.0f64).into_value()).unwrap();
    assert!(back == 0.0 && back.is_sign_negative());
}

#[test]
fn none_becomes_null_and_null_becomes_none() {
    assert_eq!(None::<i32>.into_value(), Value::Null);
    assert_eq!(Some(5i32).into_value(), Value::I32(5));
    assert_eq!(Option::<i32>::from_value(Value::Null).unwrap(), None);
    assert_eq!(Option::<i32>::from_value(Value::I32(7)).unwrap(), Some(7));
}

#[test]
fn null_into_non_option_is_conversion_error() {
    assert_conversion::<i32>(Value::Null, "i32", "null");
    assert_conversion::<String>(Value::Null, "String", "null");
}

#[test]
fn option_still_rejects_wrong_type() {
    assert_conversion::<Option<i32>>(Value::String("1".into()), "i32", "String");
}

#[test]
fn lossless_integer_widening_is_allowed() {
    assert_eq!(
        i16::from_value(Value::I8(i8::MIN)).unwrap(),
        i16::from(i8::MIN)
    );
    assert_eq!(i16::from_value(Value::U8(u8::MAX)).unwrap(), 255);
    assert_eq!(i32::from_value(Value::I16(i16::MIN)).unwrap(), -32768);
    assert_eq!(i32::from_value(Value::U16(u16::MAX)).unwrap(), 65535);
    assert_eq!(
        i64::from_value(Value::I32(i32::MIN)).unwrap(),
        i64::from(i32::MIN)
    );
    assert_eq!(
        i64::from_value(Value::U32(u32::MAX)).unwrap(),
        i64::from(u32::MAX)
    );
    assert_eq!(
        i128::from_value(Value::I64(i64::MIN)).unwrap(),
        i128::from(i64::MIN)
    );
    assert_eq!(
        i128::from_value(Value::U64(u64::MAX)).unwrap(),
        i128::from(u64::MAX)
    );
    assert_eq!(u16::from_value(Value::U8(u8::MAX)).unwrap(), 255);
    assert_eq!(u32::from_value(Value::U16(u16::MAX)).unwrap(), 65535);
    assert_eq!(
        u64::from_value(Value::U32(u32::MAX)).unwrap(),
        u64::from(u32::MAX)
    );
    assert_eq!(
        u128::from_value(Value::U64(u64::MAX)).unwrap(),
        u128::from(u64::MAX)
    );
}

#[test]
fn float_widening_is_allowed() {
    assert_eq!(f64::from_value(Value::F32(1.5)).unwrap(), 1.5);
    assert_eq!(
        f64::from_value(Value::F32(f32::INFINITY)).unwrap(),
        f64::INFINITY
    );
    assert!(f64::from_value(Value::F32(f32::NAN)).unwrap().is_nan());
}

#[test]
fn narrowing_is_conversion_error_even_when_value_fits() {
    assert_conversion::<i8>(Value::I16(1), "i8", "i16");
    assert_conversion::<i32>(Value::I64(1), "i32", "i64");
    assert_conversion::<i64>(Value::I128(1), "i64", "i128");
    assert_conversion::<u32>(Value::U64(1), "u32", "u64");
    assert_conversion::<f32>(Value::F64(1.0), "f32", "f64");
}

#[test]
fn signedness_changes_are_conversion_errors() {
    assert_conversion::<u8>(Value::I8(1), "u8", "i8");
    assert_conversion::<u64>(Value::I64(1), "u64", "i64");
    assert_conversion::<u128>(Value::I8(0), "u128", "i8");
    // Same width, unsigned into signed, would overflow.
    assert_conversion::<i32>(Value::U32(u32::MAX), "i32", "u32");
    assert_conversion::<i64>(Value::U64(u64::MAX), "i64", "u64");
}

#[test]
fn int_float_and_decimal_crossings_are_conversion_errors() {
    assert_conversion::<f64>(Value::I32(1), "f64", "i32");
    assert_conversion::<i64>(Value::F64(1.0), "i64", "f64");
    assert_conversion::<rust_decimal::Decimal>(Value::I64(1), "Decimal", "i64");
    assert_conversion::<f64>(Value::Decimal(rust_decimal::Decimal::ONE), "f64", "Decimal");
}

#[test]
fn non_numeric_mismatches_are_conversion_errors() {
    assert_conversion::<bool>(Value::I8(1), "bool", "i8");
    assert_conversion::<String>(Value::Char('a'), "String", "char");
    assert_conversion::<char>(Value::String("a".into()), "char", "String");
    assert_conversion::<chrono::NaiveDateTime>(
        Value::TimestampTz(chrono::DateTime::<chrono::Utc>::MIN_UTC),
        "Timestamp",
        "DateTime<Utc>",
    );
    assert_conversion::<uuid::Uuid>(
        Value::String(uuid::Uuid::nil().to_string()),
        "Uuid",
        "String",
    );
    assert_conversion::<serde_json::Value>(Value::String("{}".into()), "Json", "String");
    assert_conversion::<Vec<u8>>(Value::String("x".into()), "Vec<u8>", "String");
}

#[test]
fn type_name_matches_conversion_names() {
    assert_eq!(Value::Null.type_name(), "null");
    assert_eq!(Value::Bool(true).type_name(), "bool");
    assert_eq!(Value::U128(0).type_name(), "u128");
    assert_eq!(
        Value::TimestampTz(chrono::DateTime::<chrono::Utc>::MIN_UTC).type_name(),
        "DateTime<Utc>"
    );
    assert_eq!(Value::Bytes(vec![]).type_name(), "Vec<u8>");
}

#[test]
fn error_display_messages() {
    assert_eq!(
        Error::invalid_query("empty insert").to_string(),
        "invalid query: empty insert"
    );
    assert_eq!(
        Error::conversion("i32", "String").to_string(),
        "type conversion failed: expected `i32`, received `String`"
    );
    assert_eq!(
        Error::decode("age", "NULL into non-Option").to_string(),
        "decode error in column `age`: NULL into non-Option"
    );
    assert_eq!(Error::NotFound.to_string(), "record not found");
}

#[test]
fn not_found_helper() {
    assert!(Error::NotFound.is_not_found());
    assert!(!Error::invalid_query("x").is_not_found());
}

#[derive(Debug)]
struct FakeDriverError;
impl std::fmt::Display for FakeDriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("fake driver failure")
    }
}
impl StdError for FakeDriverError {}

#[test]
fn driver_error_keeps_source_and_can_be_downcast() {
    let err = Error::driver(FakeDriverError);
    let source = err.source().expect("driver error must expose its source");
    assert!(source.downcast_ref::<FakeDriverError>().is_some());
    match err {
        Error::Driver(inner) => assert!(inner.downcast_ref::<FakeDriverError>().is_some()),
        other => panic!("expected Driver, got {other:?}"),
    }
}

#[test]
fn boxed_error_converts_into_driver() {
    let boxed: Box<dyn StdError + Send + Sync> = Box::new(FakeDriverError);
    assert!(matches!(Error::from(boxed), Error::Driver(_)));
}

#[test]
fn non_driver_errors_have_no_source() {
    assert!(Error::NotFound.source().is_none());
    assert!(Error::conversion("a", "b").source().is_none());
    assert!(Error::decode("c", "m").source().is_none());
}

#[test]
fn error_is_send_sync_and_static() {
    fn check<T: Send + Sync + 'static>() {}
    check::<Error>();
}

#[test]
fn driver_message_is_not_repeated_in_display() {
    // Display + source() should not print the driver's message twice in error reporters.
    let err = Error::driver(FakeDriverError);
    let display = err.to_string();
    let source = err.source().unwrap().to_string();
    assert!(
        !display.contains(&source),
        "Display repeats the source message: {display:?}"
    );
}
