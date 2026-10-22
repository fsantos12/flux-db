//! `INSERT` query builder, single-row and bulk.
//!
//! Every row is built with [`InsertRow`]; all rows must set the same columns
//! in the same order. Checked by [`Insert::validate`].

use crate::ast::expr::Expr;
use crate::column::{Column, TypedColumn};
use crate::error::{Error, Result};
use crate::value::IntoValue;

/// One row of values for an [`Insert`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InsertRow {
    /// Columns, in call order.
    pub columns: Vec<Column>,
    /// Values, one per column.
    pub values: Vec<Expr>,
}

impl InsertRow {
    /// An empty row.
    pub fn new() -> Self {
        Self::default()
    }

    /// `column = value`, typed: `value` must match the column's Rust type.
    pub fn set_value<E, T: IntoValue>(self, column: TypedColumn<E, T>, value: T) -> Self {
        self.set(column, Expr::value(value))
    }

    /// `column = expr`, for any expression (`Expr::now()`, `Raw`...).
    pub fn set(mut self, column: impl Into<Column>, value: impl Into<Expr>) -> Self {
        self.columns.push(column.into());
        self.values.push(value.into());
        self
    }
}

/// An `INSERT INTO table (...) VALUES (...), (...)` query.
#[derive(Debug, Clone, PartialEq)]
pub struct Insert {
    /// Target table.
    pub table: &'static str,
    /// Rows to insert, in order.
    pub rows: Vec<InsertRow>,
    /// `RETURNING` columns or expressions.
    pub returning: Vec<Expr>,
}

impl Insert {
    /// Starts an `INSERT INTO table`.
    pub fn into(table: &'static str) -> Self {
        Self {
            table,
            rows: Vec::new(),
            returning: Vec::new(),
        }
    }

    /// Adds one row.
    pub fn row(mut self, row: InsertRow) -> Self {
        self.rows.push(row);
        self
    }

    /// Adds many rows (bulk insert).
    pub fn rows(mut self, rows: impl IntoIterator<Item = InsertRow>) -> Self {
        self.rows.extend(rows);
        self
    }

    /// Adds a `RETURNING` column or expression.
    pub fn returning(mut self, expr: impl Into<Expr>) -> Self {
        self.returning.push(expr.into());
        self
    }

    /// Columns shared by every row (taken from the first one).
    pub fn columns(&self) -> &[Column] {
        self.rows.first().map_or(&[], |row| &row.columns)
    }

    /// Fails with [`Error::InvalidQuery`] on an empty insert or mismatched rows.
    pub fn validate(&self) -> Result<()> {
        let Some(first) = self.rows.first() else {
            return Err(Error::invalid_query(format!("INSERT INTO {} has no rows", self.table)));
        };
        if first.columns.is_empty() {
            return Err(Error::invalid_query(format!("INSERT INTO {} has no columns", self.table)));
        }
        if let Some(index) = self.rows.iter().position(|row| row.columns != first.columns) {
            return Err(Error::invalid_query(format!(
                "INSERT INTO {}: row {index} sets different columns than row 0",
                self.table
            )));
        }
        Ok(())
    }
}