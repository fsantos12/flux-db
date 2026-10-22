//! `DELETE` query builder.
//!
//! Typestate: a `Delete` starts `Unfiltered` and can only be executed after
//! `.filter(...)` (→ `Filtered`) or `.all()` (→ `All`). `.all()` and filters
//! are mutually exclusive at compile time.
use std::marker::PhantomData;
use crate::ast::expr::Expr;
use crate::query::state::{All, Filtered, Unfiltered};

/// A `DELETE FROM table` query in typestate `S`.
#[derive(Debug, Clone, PartialEq)]
pub struct Delete<S = Unfiltered> {
    pub table: &'static str,
    pub filter: Option<Expr>,
    pub returning: Vec<Expr>,
    _state: PhantomData<S>,
}

impl<S> Delete<S> {
    /// Moves the builder to another state, keeping its data.
    fn into_state<T>(self) -> Delete<T> {
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
    pub fn from(table: &'static str) -> Self {
        Self {
            table,
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

impl Delete<Filtered> {
    /// Adds another filter, combined with `AND`.
    pub fn filter(mut self, expr: impl Into<Expr>) -> Self {
        let expr = expr.into();
        self.filter = self.filter.take().map(|existing| existing.and(expr));
        self.into_state()
    }

    /// Adds a `RETURNING` column or expression.
    pub fn returning(mut self, expr: impl Into<Expr>) -> Self {
        self.returning.push(expr.into());
        self
    }
}

impl Delete<All> {
    /// Adds a `RETURNING` column or expression.
    pub fn returning(mut self, expr: impl Into<Expr>) -> Self {
        self.returning.push(expr.into());
        self
    }
}
