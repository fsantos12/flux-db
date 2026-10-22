//! `SELECT` query builder.
//!
//! # Examples
//!
//! ```
//! use flux_db_core::column::TypedColumn;
//! use flux_db_core::query::select::Select;
//!
//! struct User;
//! const NAME: TypedColumn<User, String> = TypedColumn::new("users", "name");
//! const AGE: TypedColumn<User, i32> = TypedColumn::new("users", "age");
//!
//! // SELECT users.name FROM users WHERE users.age >= 18 ORDER BY users.name ASC LIMIT 10
//! let query = Select::from("users")
//!     .column(NAME)
//!     .filter(AGE.ge(18))
//!     .order_by(NAME.asc())
//!     .limit(10);
//! assert_eq!(query.limit, Some(10));
//! ```

use crate::ast::expr::Expr;
use crate::ast::order::OrderBy;

/// A `SELECT` query. An empty `columns` list renders as `SELECT *`.
#[derive(Debug, Clone, PartialEq)]
pub struct Select {
    /// Source table.
    pub table: &'static str,
    /// Projected columns or expressions; empty means `*`.
    pub columns: Vec<Expr>,
    /// `WHERE` clause, if any.
    pub filter: Option<Expr>,
    /// `ORDER BY` terms, in call order.
    pub order: Vec<OrderBy>,
    /// Maximum number of rows to return.
    pub limit: Option<u64>,
    /// Number of rows to skip.
    pub offset: Option<u64>,
}

impl Select {
    /// Starts a `SELECT * FROM table`.
    pub fn from(table: &'static str) -> Self {
        Self {
            table,
            columns: Vec::new(),
            filter: None,
            order: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    /// Adds one projected column or expression.
    pub fn column(mut self, expr: impl Into<Expr>) -> Self {
        self.columns.push(expr.into());
        self
    }

    /// Adds several projected columns or expressions.
    pub fn columns<I>(mut self, exprs: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Expr>,
    {
        self.columns.extend(exprs.into_iter().map(Into::into));
        self
    }

    /// Adds a filter. Repeated calls are combined with `AND`.
    pub fn filter(mut self, expr: impl Into<Expr>) -> Self {
        let expr = expr.into();
        self.filter = Some(match self.filter.take() {
            Some(existing) => existing.and(expr),
            None => expr,
        });
        self
    }

    /// Adds an `ORDER BY` term. Repeated calls keep their order.
    pub fn order_by(mut self, order: OrderBy) -> Self {
        self.order.push(order);
        self
    }

    /// Sets `LIMIT`; a later call replaces the earlier one.
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets `OFFSET`; a later call replaces the earlier one.
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
}
