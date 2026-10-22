//! `SELECT` query builder.

use crate::ast::expr::Expr;
use crate::ast::order::OrderBy;

/// A `SELECT` query. An empty `columns` list renders as `SELECT *`.
#[derive(Debug, Clone, PartialEq)]
pub struct Select {
    pub table: &'static str,
    pub columns: Vec<Expr>,
    pub filter: Option<Expr>,
    pub order: Vec<OrderBy>,
    pub limit: Option<u64>,
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

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
}