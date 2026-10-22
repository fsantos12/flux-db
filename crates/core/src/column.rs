//! Column references, typed and untyped.
use std::marker::PhantomData;

/// A reference to a column, optionally qualified by its table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Column {
    table: Option<&'static str>,
    name: &'static str,
}

impl Column {
    pub const fn new(name: &'static str) -> Self {
        Self {
            table: None,
            name,
        }
    }

    pub const fn qualified(table: &'static str, name: &'static str) -> Self {
        Self { table: Some(table), name }
    }

    pub const fn name(&self) -> &'static str { self.name }
    pub const fn table(&self) -> Option<&'static str> { self.table }
}

/// A column bound to entity `E` with Rust type `T`.
pub struct TypedColumn<E, T> {
    column: Column,
    _marker: PhantomData<fn() -> (E, T)>,
}

impl<E, T> TypedColumn<E, T> {
    pub const fn new(table: &'static str, name: &'static str) -> Self {
        Self { column: Column::qualified(table, name), _marker: PhantomData }
    }
    pub const fn column(&self) -> Column { self.column }
}

impl<E, T> Clone for TypedColumn<E, T> { fn clone(&self) -> Self { *self } }
impl<E, T> Copy for TypedColumn<E, T> {}