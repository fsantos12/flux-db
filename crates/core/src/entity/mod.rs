//! Entity traits: table metadata, row decoding and primary keys.
//!
//! These traits are implemented by `#[derive(Entity)]`; core only defines them.

use std::future::Future;

use crate::ast::expr::Expr;
use crate::driver::Executor;
use crate::query::{
    IntoQuery, Query, delete::Delete, insert::Insert, insert::InsertRow, select::Select,
    update::Update,
};
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
    /// All columns, in field order. [`Entity::values`] uses a different order
    /// (keys first); INSERT names its columns, so the two orders never conflict.
    const COLUMNS: &'static [EntityColumnDef];

    /// Primary key column/value pairs, in [`Entity::PRIMARY_KEY`] order.
    /// Empty when the entity has no key.
    ///
    /// A hand-written impl must return every key column: [`Entity::own_key_filter`]
    /// trusts this list, so a missing key widens the WHERE of update and delete.
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
    /// Errors with [`Error::InvalidQuery`] when the entity has no key, a key
    /// value is NULL, or [`Entity::primary_key_values`] does not match
    /// [`Entity::PRIMARY_KEY`] in count and column names.
    fn own_key_filter(&self) -> Result<Expr> {
        let keys = self.primary_key_values();
        if keys.is_empty() {
            return Err(no_primary_key(Self::TABLE));
        }
        let matches_definition = keys.len() == Self::PRIMARY_KEY.len()
            && keys
                .iter()
                .zip(Self::PRIMARY_KEY)
                .all(|((column, _), def)| column.name() == def.name);
        if !matches_definition {
            return Err(Error::invalid_query(format!(
                "entity `{}`: primary_key_values() does not match PRIMARY_KEY",
                Self::TABLE
            )));
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
    /// `SELECT` of every column: [`Entity::PRIMARY_KEY`] first, then the
    /// non-key [`Entity::COLUMNS`], matching the order of [`Entity::values`].
    fn select() -> Select
    where
        Self: Sized,
    {
        let non_keys = Self::COLUMNS.iter().filter(|def| !def.primary_key);
        Select::from(Self::TABLE).columns(
            Self::PRIMARY_KEY
                .iter()
                .chain(non_keys)
                .map(EntityColumnDef::column::<Self>),
        )
    }

    /// Loads the row with the given primary key, or `None` when there is none.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidQuery`] for a bad key (see
    /// [`Entity::primary_key_filter`]), or any error from the executor or
    /// [`FromRow`].
    fn find(
        db: &impl Executor,
        key: Vec<Value>,
    ) -> impl Future<Output = Result<Option<Self>>> + Send
    where
        Self: Sized,
    {
        let query = Self::primary_key_filter(key)
            .and_then(|filter| Self::select().filter(filter).limit(1).into_query());
        async move {
            let row = db.fetch_one_optional(&query?).await?;
            row.map(|row| Self::from_row(&row)).transpose()
        }
    }

    /// Loads every row of the table.
    ///
    /// # Errors
    ///
    /// Returns any error from the executor or [`FromRow`].
    fn all(db: &impl Executor) -> impl Future<Output = Result<Vec<Self>>> + Send
    where
        Self: Sized,
    {
        let query = Self::select().into_query();
        async move {
            db.fetch_all(&query?)
                .await?
                .iter()
                .map(Self::from_row)
                .collect()
        }
    }

    /// Inserts this entity with [`Entity::values`]; returns the affected rows.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidQuery`] when the INSERT is invalid, or any
    /// error from the executor.
    fn insert(&self, db: &impl Executor) -> impl Future<Output = Result<u64>> + Send {
        let row = self
            .values()
            .into_iter()
            .fold(InsertRow::new(), |row, (column, value)| {
                row.set(column, value)
            });
        let query = Insert::into(Self::TABLE).row(row).into_query();
        run(db, query)
    }

    /// Updates the non-key columns of this entity's row; returns the affected rows.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidQuery`] when the entity has no non-key columns
    /// or its key is invalid (see [`Entity::own_key_filter`]), or any error
    /// from the executor.
    fn update(&self, db: &impl Executor) -> impl Future<Output = Result<u64>> + Send {
        let query = update_query(self);
        run(db, query)
    }

    /// Deletes this entity's row; returns the affected rows.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidQuery`] when the key is invalid (see
    /// [`Entity::own_key_filter`]), or any error from the executor.
    fn delete(&self, db: &impl Executor) -> impl Future<Output = Result<u64>> + Send {
        let query = self
            .own_key_filter()
            .and_then(|filter| Delete::from(Self::TABLE).filter(filter).into_query());
        run(db, query)
    }
}

/// The UPDATE behind [`Entity::update`].
fn update_query<E: Entity>(entity: &E) -> Result<Query> {
    let columns = entity.column_values();
    if columns.is_empty() {
        return Err(Error::invalid_query(format!(
            "entity `{}` has no columns to update",
            E::TABLE
        )));
    }
    let update = Update::table(E::TABLE).filter(entity.own_key_filter()?);
    columns
        .into_iter()
        .fold(update, |update, (column, value)| update.set(column, value))
        .into_query()
}

/// Executes a prepared statement, passing a build error through the future.
async fn run(db: &impl Executor, query: Result<Query>) -> Result<u64> {
    db.execute(&query?).await
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
