use crate::types::{column::Column, row::FromRow, value::Value};

pub struct EntityColumnDef {
    pub name: &'static str,
    pub primary_key: bool,
}

pub trait PrimaryKey {
    fn into_values(self) -> Vec<Value>;
}

pub trait Entity: FromRow + Send + Sync {
    const TABLE: &'static str;
    const PRIMARY_KEY: &'static [EntityColumnDef];
    const COLUMNS: &'static [EntityColumnDef];

    fn values(&self) -> Vec<(Column, Value)>;
}