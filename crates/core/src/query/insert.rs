//! `INSERT` query builder, single-row and bulk.
//!
//! Every row is built with [`InsertRow`]; all rows must set the same columns
//! in the same order. Checked by [`Insert::validate`].
//!
//! # Examples
//!
//! ```
//! use flux_db_core::ast::expr::Expr;
//! use flux_db_core::column::TypedColumn;
//! use flux_db_core::query::insert::{Insert, InsertRow};
//!
//! struct User;
//! const NAME: TypedColumn<User, String> = TypedColumn::new("users", "name");
//! const CREATED: TypedColumn<User, i64> = TypedColumn::new("users", "created_at");
//!
//! let row = |name: &str| InsertRow::new().set_value(NAME, name.to_owned()).set(CREATED, Expr::now());
//! let query = Insert::into("users").rows([row("ann"), row("bob")]);
//! assert!(query.validate().is_ok());
//! assert_eq!(query.columns().len(), 2);
//! ```

use crate::ast::expr::Expr;
use crate::types::column::{Column, TypedColumn};
use crate::types::error::{Error, Result};
use crate::query::duplicate_column;
use crate::types::value::IntoValue;

/// One row of values for an [`Insert`].
///
/// Fields are private so `columns` and `values` always have the same length.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InsertRow {
    columns: Vec<Column>,
    values: Vec<Expr>,
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

    /// Columns, in call order.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// Values, one per column, in the same order as [`columns`](Self::columns).
    pub fn values(&self) -> &[Expr] {
        &self.values
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

    /// Checks that the insert can be rendered.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidQuery`] when there are no rows, the first row
    /// sets no columns or the same column twice, or a row sets different
    /// columns than row 0.
    pub fn validate(&self) -> Result<()> {
        let Some(first) = self.rows.first() else {
            return Err(Error::invalid_query(format!(
                "INSERT INTO {} has no rows",
                self.table
            )));
        };
        if first.columns.is_empty() {
            return Err(Error::invalid_query(format!(
                "INSERT INTO {} has no columns",
                self.table
            )));
        }
        // Other rows must match row 0, so checking it is enough.
        if let Some(name) = duplicate_column(&first.columns) {
            return Err(Error::invalid_query(format!(
                "INSERT INTO {}: column {name} is set more than once",
                self.table
            )));
        }
        if let Some(index) = self
            .rows
            .iter()
            .position(|row| row.columns != first.columns)
        {
            return Err(Error::invalid_query(format!(
                "INSERT INTO {}: row {index} sets different columns than row 0",
                self.table
            )));
        }
        Ok(())
    }
}
