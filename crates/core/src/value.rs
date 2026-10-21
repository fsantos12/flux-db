//! Database values and conversions between Rust types and [`Value`].

/// A database value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {}

/// Converts a Rust type into a [`Value`].
pub trait IntoValue {}

/// Converts a [`Value`] back into a Rust type.
pub trait FromValue: Sized {}
