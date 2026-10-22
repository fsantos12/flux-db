//! Column references, typed and untyped.

use std::marker::PhantomData;

/// A reference to a column, optionally qualified by its table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Column {
    table: Option<&'static str>,
    name: &'static str,
}

impl Column {
    /// Creates an unqualified column reference.
    pub const fn new(name: &'static str) -> Self {
        Self { table: None, name }
    }

    /// Creates a column reference qualified by `table`.
    ///
    /// Names are kept as single identifiers: a dot in `name` is never split.
    pub const fn qualified(table: &'static str, name: &'static str) -> Self {
        Self {
            table: Some(table),
            name,
        }
    }

    /// Column name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Table qualifier, if any.
    pub const fn table(&self) -> Option<&'static str> {
        self.table
    }
}

impl<E, T> From<TypedColumn<E, T>> for Column {
    fn from(column: TypedColumn<E, T>) -> Self {
        column.column()
    }
}

/// A column bound to entity `E` with Rust type `T`.
///
/// `T` drives which helpers are available (e.g. `is_null` only on
/// `Option<T>`, LIKE helpers only on `String`). It is a zero-cost marker:
/// `TypedColumn` is always `Copy`, whatever `E` and `T` are.
///
/// # Examples
///
/// ```
/// use flux_db_core::types::column::TypedColumn;
///
/// struct User;
/// const AGE: TypedColumn<User, i32> = TypedColumn::new("users", "age");
///
/// assert_eq!(AGE.column().table(), Some("users"));
/// assert_eq!(AGE.column().name(), "age");
/// ```
pub struct TypedColumn<E, T> {
    column: Column,
    _marker: PhantomData<fn() -> (E, T)>,
}

impl<E, T> TypedColumn<E, T> {
    /// Creates a typed column qualified by `table`.
    pub const fn new(table: &'static str, name: &'static str) -> Self {
        Self {
            column: Column::qualified(table, name),
            _marker: PhantomData,
        }
    }

    /// The untyped column reference.
    pub const fn column(&self) -> Column {
        self.column
    }
}

// Manual impls: a derive would wrongly require `E: Clone` and `T: Clone`.
impl<E, T> Clone for TypedColumn<E, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<E, T> Copy for TypedColumn<E, T> {}
