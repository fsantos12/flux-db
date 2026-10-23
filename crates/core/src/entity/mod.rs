use crate::types::{column::Column, row::FromRow, value::Value};

pub struct EntityColumn {
    pub name: &'static str,
    pub primary_key: bool,
}

pub trait PrimaryKey {
    fn into_values(self) -> Vec<Value>;
}

pub trait Entity: FromRow + Send + Sync {
    const TABLE: &'static str;
    const PRIMARY_KEY: &'static [EntityColumn];
    const COLUMNS: &'static [EntityColumn];

    fn values(&self) -> Vec<(Column, Value)>;
}