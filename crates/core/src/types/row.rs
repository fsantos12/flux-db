//! A single result row returned by a query.

use std::sync::Arc;

use crate::types::error::{Error, Result};
use crate::types::value::{FromValue, Value};

pub trait FromRow: Sized {
    fn from_row(row: &Row) -> Result<Self>;
}

/// The untyped row itself, so shortcuts can return plain [`Row`]s.
impl FromRow for Row {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(row.clone())
    }
}

/// An ordered set of column names and their values.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Row {
    columns: Arc<[Box<str>]>,
    values: Vec<Value>,
}

impl Row {
    /// Creates a row. Built by drivers; `columns` is shared across all rows of one result.
    pub fn new(columns: Arc<[Box<str>]>, values: Vec<Value>) -> Self {
        debug_assert_eq!(columns.len(), values.len(), "column/value count mismatch");
        Self { columns, values }
    }

    /// Number of values in the row.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns `true` if the row has no values.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Column names, in order.
    pub fn columns(&self) -> &[Box<str>] {
        &self.columns
    }

    /// Raw value by column name.
    pub fn value(&self, column: &str) -> Option<&Value> {
        self.index_of(column).map(|i| &self.values[i])
    }

    /// Raw value by position.
    pub fn value_at(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    /// Typed value by column name. Failures become a column-scoped [`Error::Decode`].
    pub fn get<T: FromValue>(&self, column: &str) -> Result<T> {
        let index = self
            .index_of(column)
            .ok_or_else(|| Error::decode(column, "column not found"))?;
        self.decode_at(index)
    }

    /// Typed value by position (fastest path, used by the entity derive).
    pub fn get_at<T: FromValue>(&self, index: usize) -> Result<T> {
        if index >= self.values.len() {
            return Err(Error::decode(
                format!("#{index}"),
                format!(
                    "index out of bounds (row has {} columns)",
                    self.values.len()
                ),
            ));
        }
        self.decode_at(index)
    }

    /// Consumes the row, yielding its owned values without cloning.
    pub fn into_values(self) -> Vec<Value> {
        self.values
    }

    /// Decodes the value at a known-valid index, tagging errors with the column name.
    fn decode_at<T: FromValue>(&self, index: usize) -> Result<T> {
        T::from_value(self.values[index].clone())
            .map_err(|err| Error::decode(&*self.columns[index], err.to_string()))
    }

    /// Position of a column by name (linear scan: rows are narrow).
    fn index_of(&self, column: &str) -> Option<usize> {
        self.columns.iter().position(|c| &**c == column)
    }
}
