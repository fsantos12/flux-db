//! Entity traits: table metadata, row decoding and primary keys.
//!
//! These traits are implemented by `#[derive(Entity)]`; core only defines them.

use crate::types::{
    column::Column,
    row::FromRow,
    value::{IntoValue, Value},
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

/// A primary key value: one value per primary key column.
///
/// Implemented for single key types and for tuples of 2 to 4 elements.
pub trait PrimaryKey {
    /// The key values, in the order of [`Entity::PRIMARY_KEY`].
    fn into_values(self) -> Vec<Value>;
}

/// A database table mapped to a Rust type.
///
/// Every entity can be queried and inserted; entities with a primary key also
/// implement [`PrimaryKeyed`].
pub trait Entity: FromRow + Send + Sync {
    /// Table name.
    const TABLE: &'static str;
    /// Primary key columns, in key order. Empty when the entity has no key.
    const PRIMARY_KEY: &'static [EntityColumnDef];
    /// All columns, in field order.
    const COLUMNS: &'static [EntityColumnDef];

    /// Column/value pairs for INSERT and UPDATE.
    fn values(&self) -> Vec<(Column, Value)>;
}

/// An entity with a primary key, enabling `find`, `update` and `delete`.
pub trait PrimaryKeyed: Entity {
    /// The key type: a single value, or a tuple for composite keys.
    type Id: PrimaryKey;

    /// The current primary key value.
    fn id(&self) -> Self::Id;
}

macro_rules! single_key {
    ($($(#[$attr:meta])* $ty:ty),* $(,)?) => {
        $(
            $(#[$attr])*
            impl PrimaryKey for $ty {
                fn into_values(self) -> Vec<Value> {
                    vec![self.into_value()]
                }
            }
        )*
    };
}

single_key! {
    i8, i16, i32, i64, i128,
    u8, u16, u32, u64, u128,
    char, String, &str, Vec<u8>,
    #[cfg(feature = "decimal")] rust_decimal::Decimal,
    #[cfg(feature = "chrono")] chrono::NaiveDate,
    #[cfg(feature = "chrono")] chrono::NaiveDateTime,
    #[cfg(feature = "chrono")] chrono::DateTime<chrono::Utc>,
    #[cfg(feature = "uuid")] uuid::Uuid,
}

macro_rules! tuple_key {
    ($($name:ident),+) => {
        impl<$($name: IntoValue),+> PrimaryKey for ($($name,)+) {
            #[allow(non_snake_case)]
            fn into_values(self) -> Vec<Value> {
                let ($($name,)+) = self;
                vec![$($name.into_value()),+]
            }
        }
    };
}

tuple_key!(A, B);
tuple_key!(A, B, C);
tuple_key!(A, B, C, D);
