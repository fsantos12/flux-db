//! Unit tests for `IntoQuery`: every builder validates before it becomes a `Query`.

use flux_db_core::ast::expr::Expr;
use flux_db_core::query::delete::Delete;
use flux_db_core::query::insert::{Insert, InsertRow};
use flux_db_core::query::raw::RawQuery;
use flux_db_core::query::select::Select;
use flux_db_core::query::update::Update;
use flux_db_core::query::{IntoQuery, Query};
use flux_db_core::types::column::TypedColumn;
use flux_db_core::types::error::Error;

struct User;
const ID: TypedColumn<User, i64> = TypedColumn::new("users", "id");
const NAME: TypedColumn<User, String> = TypedColumn::new("users", "name");

fn invalid(r: flux_db_core::types::error::Result<Query>) -> bool {
    matches!(r, Err(Error::InvalidQuery(_)))
}

#[test]
fn select_and_raw_pass_through_unchanged() {
    let s = Select::from("users").filter(ID.eq(1));
    assert_eq!(s.clone().into_query().unwrap(), Query::Select(s));
    let r = RawQuery::new("SELECT 1");
    assert_eq!(r.clone().into_query().unwrap(), Query::Raw(r));
}

#[test]
fn query_into_query_is_identity() {
    let q = Select::from("users").into_query().unwrap();
    assert_eq!(q.clone().into_query().unwrap(), q);
}

#[test]
fn empty_insert_is_rejected() {
    assert!(invalid(Insert::into("users").into_query()));
    assert!(invalid(
        Insert::into("users").row(InsertRow::new()).into_query()
    ));
}

#[test]
fn insert_with_duplicate_column_is_rejected() {
    let row = InsertRow::new()
        .set_value(NAME, "a".into())
        .set_value(NAME, "b".into());
    assert!(invalid(Insert::into("users").row(row).into_query()));
}

#[test]
fn valid_insert_is_kept_as_is() {
    let q = Insert::into("users").row(InsertRow::new().set_value(NAME, "a".into()));
    assert_eq!(q.clone().into_query().unwrap(), Query::Insert(q));
}

#[test]
fn update_without_assignments_is_rejected_in_executable_states() {
    assert!(invalid(Update::table("users").all().into_query()));
    assert!(invalid(
        Update::table("users").filter(ID.eq(1)).into_query()
    ));
}

#[test]
fn filtered_update_keeps_table_assignments_filter_and_returning() {
    let filter = ID.eq(1);
    let q = Update::table("users")
        .set_value(NAME, "x".into())
        .filter(filter.clone())
        .returning(ID);
    let Query::Update(u) = q.into_query().unwrap() else {
        panic!("expected Update")
    };
    assert_eq!(u.table, "users");
    assert_eq!(u.assignments.len(), 1);
    assert_eq!(u.filter, Some(filter));
    assert_eq!(u.returning, vec![Expr::from(ID)]);
}

#[test]
fn update_all_has_no_filter() {
    let q = Update::table("users").set_value(NAME, "x".into()).all();
    let Query::Update(u) = q.into_query().unwrap() else {
        panic!("expected Update")
    };
    assert_eq!(u.filter, None);
}

#[test]
fn delete_filtered_and_all_convert() {
    let filter = ID.eq(1);
    let Query::Delete(d) = Delete::from("users")
        .filter(filter.clone())
        .into_query()
        .unwrap()
    else {
        panic!("expected Delete")
    };
    assert_eq!(d.filter, Some(filter));
    let Query::Delete(d) = Delete::from("users")
        .all()
        .returning(ID)
        .into_query()
        .unwrap()
    else {
        panic!("expected Delete")
    };
    assert_eq!((d.filter, d.returning.len()), (None, 1));
}
