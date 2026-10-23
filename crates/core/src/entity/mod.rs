//! Entity traits: table metadata, row decoding and primary keys.
//!
//! These traits are implemented by `#[derive(Entity)]`; core only defines them.

use crate::ast::expr::Expr;
use crate::types::{
    column::Column,
    error::{Error, Result},
    row::FromRow,
    value::Value,
};

/// Static metadata for one entity column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityColumnDef {
    /// Column name in the database.
    pub name: &'static str,
    /// Whether the column is part of the primary key.
    pub primary_key: bool,
}

impl EntityColumnDef {
    /// The column qualified by the entity's table.
    pub fn column<E: Entity>(&self) -> Column {
        Column::qualified(E::TABLE, self.name)
    }
}

/// A database table mapped to a Rust type.
///
/// Entities with a non-empty [`Entity::PRIMARY_KEY`] can also be found, updated
/// and deleted through [`Entity::primary_key_filter`].
pub trait Entity: FromRow + Send + Sync {
    /// Table name.
    const TABLE: &'static str;
    /// Primary key columns, in key order. Empty when the entity has no key.
    const PRIMARY_KEY: &'static [EntityColumnDef];
    /// All columns, in field order.
    const COLUMNS: &'static [EntityColumnDef];

    /// Primary key column/value pairs, in [`Entity::PRIMARY_KEY`] order.
    /// Empty when the entity has no key.
    fn primary_key_values(&self) -> Vec<(Column, Value)>;

    /// Non-key column/value pairs, in field order (the SET list of an UPDATE).
    fn column_values(&self) -> Vec<(Column, Value)>;

    /// All column/value pairs for an INSERT: keys first, then the other columns.
    fn values(&self) -> Vec<(Column, Value)> {
        let mut values = self.primary_key_values();
        values.extend(self.column_values());
        values
    }

    /// `pk1 = v1 AND pk2 = v2 ...` for this entity's own key (update, delete).
    ///
    /// Errors with [`Error::InvalidQuery`] when the entity has no key or a key
    /// value is NULL.
    fn own_key_filter(&self) -> Result<Expr> {
        let keys = self.primary_key_values();
        if keys.is_empty() {
            return Err(no_primary_key(Self::TABLE));
        }
        keys.into_iter()
            .map(|(column, value)| key_equals(column, value))
            .collect::<Result<Vec<_>>>()
            .map(Expr::all)
    }

    /// `pk1 = v1 AND pk2 = v2 ...` from bare key values (find).
    ///
    /// Errors with [`Error::InvalidQuery`] when the entity has no key, the
    /// number of values differs from [`Entity::PRIMARY_KEY`], or a value is NULL.
    fn primary_key_filter(values: Vec<Value>) -> Result<Expr>
    where
        Self: Sized,
    {
        if Self::PRIMARY_KEY.is_empty() {
            return Err(no_primary_key(Self::TABLE));
        }
        if values.len() != Self::PRIMARY_KEY.len() {
            return Err(Error::invalid_query(format!(
                "entity `{}` expects {} primary key values, got {}",
                Self::TABLE,
                Self::PRIMARY_KEY.len(),
                values.len()
            )));
        }
        Self::PRIMARY_KEY
            .iter()
            .zip(values)
            .map(|(def, value)| key_equals(def.column::<Self>(), value))
            .collect::<Result<Vec<_>>>()
            .map(Expr::all)
    }
}

/// Error for a key-based operation on an entity without a primary key.
fn no_primary_key(table: &str) -> Error {
    Error::invalid_query(format!("entity `{table}` has no primary key"))
}

/// `column = value` for one key part; NULL is rejected because `= NULL` never matches.
fn key_equals(column: Column, value: Value) -> Result<Expr> {
    if matches!(value, Value::Null) {
        return Err(Error::invalid_query(format!(
            "primary key column `{}` cannot be NULL",
            column.name()
        )));
    }
    Ok(Expr::from(column).equals(value))
}
