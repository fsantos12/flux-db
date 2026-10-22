//! Raw SQL escape hatch: the SQL text is sent as-is, and parameters are always bound.

use crate::types::value::{IntoValue, Value};

/// A full raw statement with bound parameters.
///
/// The SQL is not parsed, checked or rewritten, so placeholders must use
/// the target database's own syntax (`$1` on Postgres, `?` on MySQL/SQLite).
/// Only `params` are safe for untrusted input.
///
/// # Examples
///
/// ```
/// use flux_db_core::query::raw::RawQuery;
/// use flux_db_core::value::Value;
///
/// let query = RawQuery::new("SELECT * FROM users WHERE id = $1 OR id = $2").bind_all([1_i64, 2]);
/// assert_eq!(query.params, vec![Value::I64(1), Value::I64(2)]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RawQuery {
    /// SQL text, sent as-is.
    pub sql: String,
    /// Parameters bound to the placeholders, in order.
    pub params: Vec<Value>,
}

impl RawQuery {
    /// A raw statement with no parameters yet.
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            params: Vec::new(),
        }
    }

    /// Binds the next parameter.
    pub fn bind(mut self, value: impl IntoValue) -> Self {
        self.params.push(value.into_value());
        self
    }

    /// Binds several parameters in order.
    pub fn bind_all<I>(mut self, values: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoValue,
    {
        self.params
            .extend(values.into_iter().map(IntoValue::into_value));
        self
    }
}
