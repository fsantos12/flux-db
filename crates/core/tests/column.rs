//! Unit tests for `Column` (static and runtime names) and `TypedColumn`.

use flux_db_core::query::insert::{Insert, InsertRow};
use flux_db_core::query::select::Select;
use flux_db_core::types::column::{Column, TypedColumn};
use flux_db_core::types::error::Error;
use flux_db_core::types::value::IntoValue;

struct User;
const AGE: TypedColumn<User, i32> = TypedColumn::new("users", "age");

// Const accessors must be usable in const context.
const AGE_NAME: &str = AGE.name();
const AGE_TABLE: &str = AGE.table();

fn is_invalid(r: flux_db_core::types::error::Result<()>) -> bool {
    matches!(r, Err(Error::InvalidQuery(_)))
}

#[test]
fn typed_column_const_accessors() {
    assert_eq!(AGE_NAME, "age");
    assert_eq!(AGE_TABLE, "users");
    assert_eq!(AGE.column(), Column::qualified("users", "age"));
    assert_eq!(Column::from(AGE), Column::qualified("users", "age"));
}

#[test]
fn static_and_dynamic_columns_with_same_name_are_equal() {
    assert_eq!(Column::new("id"), Column::dynamic(String::from("id")));
    assert_eq!(
        Column::qualified("users", "id"),
        Column::dynamic_qualified(String::from("users"), String::from("id"))
    );
    assert_ne!(Column::new("id"), Column::qualified("users", "id"));
}

#[test]
fn dynamic_column_accessors() {
    let c = Column::dynamic_qualified(String::from("t"), format!("c{}", 1));
    assert_eq!(c.name(), "c1");
    assert_eq!(c.table(), Some("t"));
    assert_eq!(Column::dynamic("x").table(), None);
}

#[test]
fn static_and_dynamic_same_name_is_duplicate_in_insert() {
    let row = InsertRow::new()
        .set(Column::new("id"), 1i64.into_value())
        .set(Column::dynamic(String::from("id")), 2i64.into_value());
    assert!(is_invalid(Insert::into("users").row(row).validate()));
}

#[test]
fn qualified_and_bare_same_name_is_duplicate_in_insert() {
    // Blueprint 01: `users.name` and bare `name` count as duplicates.
    let row = InsertRow::new()
        .set(Column::qualified("users", "name"), "a".into_value())
        .set(Column::new("name"), "b".into_value());
    assert!(is_invalid(Insert::into("users").row(row).validate()));
}

#[test]
fn empty_dynamic_name_is_accepted_in_core() {
    // Core accepts any name; rejection with InvalidQuery belongs to sql-render.
    let c = Column::dynamic(String::new());
    assert_eq!(c.name(), "");
    let row = InsertRow::new().set(c, 1i64.into_value());
    assert!(Insert::into("users").row(row).validate().is_ok());
}

#[test]
fn dynamic_table_names_in_builders() {
    let t = format!("tenant_{}", 7);
    let q = Insert::into(t.clone()).row(InsertRow::new().set(Column::new("id"), 1i64.into_value()));
    assert!(q.validate().is_ok());
    let _ = Select::from(t);
}
