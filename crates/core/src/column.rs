//! Column references, typed and untyped.

use std::marker::PhantomData;

/// A reference to a column, optionally qualified by its table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {}

/// A column bound to entity `E` with Rust type `T`.
#[derive(Debug)]
pub struct TypedColumn<E, T> {
    _marker: PhantomData<fn() -> (E, T)>,
}
