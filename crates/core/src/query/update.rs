//! `UPDATE` query builder.
//!
//! Same typestate as [`Delete`](crate::query::delete::Delete): executable only
//! after `.filter(...)` or `.all()`. At least one assignment is checked by
//! [`Update::validate`] when the update is turned into a query.
//!
//! # Examples
//!
//! ```
//! use flux_db_core::column::TypedColumn;
//! use flux_db_core::query::update::Update;
//!
//! struct User;
//! const ID: TypedColumn<User, i64> = TypedColumn::new("users", "id");
//! const NAME: TypedColumn<User, String> = TypedColumn::new("users", "name");
//!
//! // UPDATE users SET name = 'ann' WHERE users.id = 1
//! let query = Update::table("users").set_value(NAME, "ann".to_owned()).filter(ID.eq(1));
//! assert!(query.validate().is_ok());
//! ```

use std::marker::PhantomData;

use crate::ast::expr::Expr;
use crate::column::{Column, TypedColumn};
use crate::error::{Error, Result};
use crate::query::duplicate_column;
use crate::query::state::{All, Executable, Filtered, Unfiltered};
use crate::value::IntoValue;

/// One `column = value` assignment in a `SET` clause.
#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    /// Column being written.
    pub column: Column,
    /// New value or expression.
    pub value: Expr,
}

/// An `UPDATE table SET ...` query in typestate `S`.
#[derive(Debug, Clone, PartialEq)]
pub struct Update<S = Unfiltered> {
    /// Target table.
    pub table: &'static str,
    /// `SET` assignments, in call order.
    pub assignments: Vec<Assignment>,
    /// `WHERE` clause; `None` only in the `Unfiltered` and `All` states.
    pub filter: Option<Expr>,
    /// `RETURNING` columns or expressions.
    pub returning: Vec<Expr>,
    _state: PhantomData<S>,
}

impl Update<Unfiltered> {
    /// Starts an `UPDATE table`. Not executable until filtered or `.all()`.
    pub fn table(table: &'static str) -> Self {
        Self {
            table,
            assignments: Vec::new(),
            filter: None,
            returning: Vec::new(),
            _state: PhantomData,
        }
    }

    /// Explicitly targets every row in the table.
    pub fn all(self) -> Update<All> {
        self.into_state()
    }

    /// Adds the first filter.
    pub fn filter(self, expr: impl Into<Expr>) -> Update<Filtered> {
        Update {
            filter: Some(expr.into()),
            ..self.into_state()
        }
    }
}

impl<S: Executable> Update<S> {
    /// Adds a `RETURNING` column or expression.
    pub fn returning(mut self, expr: impl Into<Expr>) -> Self {
        self.returning.push(expr.into());
        self
    }
}

impl Update<Filtered> {
    /// Adds another filter, combined with `AND`.
    pub fn filter(mut self, expr: impl Into<Expr>) -> Self {
        let expr = expr.into();
        self.filter = self.filter.take().map(|existing| existing.and(expr));
        self
    }
}

impl<S> Update<S> {
    /// `column = value`, typed: `value` must match the column's Rust type.
    pub fn set_value<E, T: IntoValue>(self, column: TypedColumn<E, T>, value: T) -> Self {
        self.set(column, Expr::value(value))
    }

    /// `column = expr`, for any expression (`Expr::now()`, another column, `Raw`...).
    pub fn set(mut self, column: impl Into<Column>, value: impl Into<Expr>) -> Self {
        self.assignments.push(Assignment {
            column: column.into(),
            value: value.into(),
        });
        self
    }

    /// Checks that the update can be rendered.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidQuery`] when there are no `SET` assignments.
    pub fn validate(&self) -> Result<()> {
        if self.assignments.is_empty() {
            return Err(Error::invalid_query(format!(
                "UPDATE {} has no SET assignments",
                self.table
            )));
        }
        if let Some(name) = duplicate_column(self.assignments.iter().map(|a| &a.column)) {
            return Err(Error::invalid_query(format!(
                "UPDATE {}: column {name} is assigned more than once",
                self.table
            )));
        }
        Ok(())
    }

    /// Moves the builder to another state, keeping its data.
    fn into_state<T>(self) -> Update<T> {
        Update {
            table: self.table,
            assignments: self.assignments,
            filter: self.filter,
            returning: self.returning,
            _state: PhantomData,
        }
    }
}
