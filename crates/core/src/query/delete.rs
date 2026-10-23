//! `DELETE` query builder.
//!
//! Typestate: a `Delete` starts `Unfiltered` and can only be executed after
//! `.filter(...)` (→ `Filtered`) or `.all()` (→ `All`). `.all()` and filters
//! are mutually exclusive at compile time.
//!
//! # Examples
//!
//! ```
//! use flux_db_core::types::column::TypedColumn;
//! use flux_db_core::query::delete::Delete;
//!
//! struct User;
//! const ID: TypedColumn<User, i64> = TypedColumn::new("users", "id");
//!
//! // DELETE FROM users WHERE users.id = 1 RETURNING users.id
//! let query = Delete::from("users").filter(ID.eq(1)).returning(ID);
//! assert!(query.filter.is_some());
//! assert_eq!(query.returning.len(), 1);
//! ```
//!
//! Executing without a filter or `.all()` does not compile:
//!
//! ```compile_fail
//! use flux_db_core::query::delete::Delete;
//!
//! let _ = Delete::from("users").returning(flux_db_core::ast::expr::Expr::count_all());
//! ```

use std::borrow::Cow;
use std::marker::PhantomData;

use crate::ast::expr::Expr;
use crate::query::state::{All, Executable, Filtered, NoReturning, Returning, Unfiltered};

/// A `DELETE FROM table` query in typestate `S`.
///
/// `R` is [`Returning`] once `.returning(...)` was called, which unlocks the
/// fetch shortcuts.
#[derive(Debug, Clone, PartialEq)]
pub struct Delete<S = Unfiltered, R = NoReturning> {
    /// Target table.
    pub table: Cow<'static, str>,
    /// `WHERE` clause; `None` only in the `Unfiltered` and `All` states.
    pub filter: Option<Expr>,
    /// `RETURNING` columns or expressions.
    pub returning: Vec<Expr>,
    _state: PhantomData<(S, R)>,
}

impl<S, R> Delete<S, R> {
    /// Moves the builder to another state, keeping its data.
    fn into_state<T, Q>(self) -> Delete<T, Q> {
        Delete {
            table: self.table,
            filter: self.filter,
            returning: self.returning,
            _state: PhantomData,
        }
    }
}

impl Delete<Unfiltered> {
    /// Starts a `DELETE FROM table`. Not executable until filtered or `.all()`.
    pub fn from(table: impl Into<Cow<'static, str>>) -> Self {
        Self {
            table: table.into(),
            filter: None,
            returning: Vec::new(),
            _state: PhantomData,
        }
    }

    /// Explicitly targets every row in the table.
    pub fn all(self) -> Delete<All> {
        self.into_state()
    }

    /// Adds the first filter.
    pub fn filter(self, expr: impl Into<Expr>) -> Delete<Filtered> {
        Delete {
            filter: Some(expr.into()),
            ..self.into_state()
        }
    }
}

impl<S: Executable, R> Delete<S, R> {
    /// Adds a `RETURNING` column or expression.
    pub fn returning(self, expr: impl Into<Expr>) -> Delete<S, Returning> {
        let mut next = self.into_state();
        next.returning.push(expr.into());
        next
    }
}

impl<R> Delete<Filtered, R> {
    /// Adds another filter, combined with `AND`.
    pub fn filter(mut self, expr: impl Into<Expr>) -> Self {
        let expr = expr.into();
        self.filter = self.filter.take().map(|existing| existing.and(expr));
        self
    }
}

impl<S: Executable, R> Delete<S, R> {
    crate::query::run::execute_shortcut!();
}

impl<S: Executable> Delete<S, Returning> {
    crate::query::run::fetch_shortcuts!();
}
