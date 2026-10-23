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

    /// Column/value pairs for INSERT and UPDATE.
    fn values(&self) -> Vec<(Column, Value)>;

    /// Current primary key values, in [`Entity::PRIMARY_KEY`] order.
    /// Empty when the entity has no key.
    fn primary_key_values(&self) -> Vec<Value>;

    /// `pk1 = $1 AND pk2 = $2 ...` for the given key values.
    ///
    /// Errors with [`Error::InvalidQuery`] when the entity has no key, the
    /// number of values differs from [`Entity::PRIMARY_KEY`], or a value is NULL.
    fn primary_key_filter(values: Vec<Value>) -> Result<Expr>
    where
        Self: Sized,
    {
        if Self::PRIMARY_KEY.is_empty() {
            return Err(Error::invalid_query(format!(
                "entity `{}` has no primary key",
                Self::TABLE
            )));
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
            .map(|(def, value)| {
                if matches!(value, Value::Null) {
                    return Err(Error::invalid_query(format!(
                        "primary key column `{}` cannot be NULL",
                        def.name
                    )));
                }
                Ok(Expr::from(def.column::<Self>()).equals(value))
            })
            .collect::<Result<Vec<_>>>()
            .map(Expr::all)
    }
}
