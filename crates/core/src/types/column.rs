//! Column references, typed and untyped.

use std::borrow::Cow;
use std::marker::PhantomData;

/// A reference to a column, optionally qualified by its table.
///
/// Static names (literals, derive output) are borrowed and never allocate;
/// names known only at runtime are owned, through [`Column::string`] and
/// [`Column::string_qualified`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Column {
    table: Option<Cow<'static, str>>,
    name: Cow<'static, str>,
}

impl Column {
    /// Creates an unqualified column reference.
    pub const fn new(name: &'static str) -> Self {
        Self {
            table: None,
            name: Cow::Borrowed(name),
        }
    }

    /// Creates a column reference qualified by `table`.
    ///
    /// Names are kept as single identifiers: a dot in `name` is never split.
    pub const fn qualified(table: &'static str, name: &'static str) -> Self {
        Self {
            table: Some(Cow::Borrowed(table)),
            name: Cow::Borrowed(name),
        }
    }

    /// Creates an unqualified column whose name is only known at runtime.
    pub fn string(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            table: None,
            name: name.into(),
        }
    }

    /// Creates a qualified column whose names are only known at runtime.
    pub fn string_qualified(
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            table: Some(table.into()),
            name: name.into(),
        }
    }

    /// Column name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Table qualifier, if any.
    pub fn table(&self) -> Option<&str> {
        self.table.as_deref()
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
    table: &'static str,
    name: &'static str,
    _marker: PhantomData<fn() -> (E, T)>,
}

impl<E, T> TypedColumn<E, T> {
    /// Creates a typed column qualified by `table`.
    pub const fn new(table: &'static str, name: &'static str) -> Self {
        Self {
            table,
            name,
            _marker: PhantomData,
        }
    }

    /// The untyped column reference (borrowed, no allocation).
    pub const fn column(&self) -> Column {
        Column::qualified(self.table, self.name)
    }

    /// Column name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Table name.
    pub const fn table(&self) -> &'static str {
        self.table
    }
}

// Manual impls: a derive would wrongly require `E: Clone` and `T: Clone`.
impl<E, T> Clone for TypedColumn<E, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<E, T> Copy for TypedColumn<E, T> {}
