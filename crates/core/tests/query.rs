//! Unit tests for the SELECT, INSERT, UPDATE, DELETE and raw query builders.

use flux_db_core::ast::expr::Expr;
use flux_db_core::query::delete::Delete;
use flux_db_core::query::insert::{Insert, InsertRow};
use flux_db_core::query::raw::RawQuery;
use flux_db_core::query::select::Select;
use flux_db_core::query::update::Update;
use flux_db_core::types::column::{Column, TypedColumn};
use flux_db_core::types::error::Error;
use flux_db_core::types::value::{IntoValue, Value};

struct User;
const ID: TypedColumn<User, i64> = TypedColumn::new("users", "id");
const AGE: TypedColumn<User, Option<i32>> = TypedColumn::new("users", "age");
const NAME: TypedColumn<User, String> = TypedColumn::new("users", "name");

fn is_invalid(r: flux_db_core::types::error::Result<()>) -> bool {
    matches!(r, Err(Error::InvalidQuery(_)))
}

// --- SELECT ---

#[test]
fn select_defaults_to_star_with_nothing_else() {
    let q = Select::from("users");
    assert!(q.columns.is_empty() && q.filter.is_none() && q.order.is_empty());
    assert_eq!((q.limit, q.offset), (None, None));
}

#[test]
fn select_filters_are_and_ed_and_order_kept() {
    let a = ID.gt(1);
    let b = NAME.eq("x".into());
    let q = Select::from("users")
        .filter(a.clone())
        .filter(b.clone())
        .order_by(NAME.asc())
        .order_by(ID.desc());
    assert_eq!(q.filter, Some(Expr::And(vec![a, b])));
    assert_eq!(q.order, vec![NAME.asc(), ID.desc()]);
}

#[test]
fn select_last_limit_and_offset_win_and_extremes_are_kept() {
    let q = Select::from("users")
        .limit(10)
        .limit(0)
        .offset(5)
        .offset(u64::MAX);
    // u64::MAX is above Postgres' bigint range: the renderer must reject it, not wrap.
    assert_eq!((q.limit, q.offset), (Some(0), Some(u64::MAX)));
}

#[test]
fn select_columns_accept_functions() {
    let q = Select::from("users")
        .column(ID)
        .columns([AGE.count(), Expr::count_all()]);
    assert_eq!(q.columns.len(), 3);
}

// --- INSERT ---

#[test]
fn insert_without_rows_is_invalid() {
    assert!(is_invalid(Insert::into("users").validate()));
}

#[test]
fn insert_with_an_empty_row_is_invalid() {
    assert!(is_invalid(
        Insert::into("users").row(InsertRow::new()).validate()
    ));
}

#[test]
fn insert_single_row_is_valid_and_keeps_call_order() {
    let q = Insert::into("users").row(
        InsertRow::new()
            .set_value(NAME, "a".into())
            .set_value(AGE, None),
    );
    assert!(q.validate().is_ok());
    assert_eq!(q.columns(), &[NAME.column(), AGE.column()]);
    assert_eq!(q.rows[0].values()[1], Expr::Value(Value::Null));
}

#[test]
fn insert_rows_with_different_column_order_is_invalid() {
    let q = Insert::into("users").rows([
        InsertRow::new()
            .set_value(NAME, "a".into())
            .set_value(AGE, Some(1)),
        InsertRow::new()
            .set_value(AGE, Some(2))
            .set_value(NAME, "b".into()),
    ]);
    assert!(is_invalid(q.validate()));
}

#[test]
fn insert_rows_with_a_missing_column_is_invalid() {
    let q = Insert::into("users").rows([
        InsertRow::new()
            .set_value(NAME, "a".into())
            .set_value(AGE, Some(1)),
        InsertRow::new().set_value(NAME, "b".into()),
    ]);
    assert!(is_invalid(q.validate()));
}

#[test]
fn insert_accepts_expressions_as_values() {
    let q = Insert::into("users").row(InsertRow::new().set(Column::new("created_at"), Expr::now()));
    assert!(q.validate().is_ok());
    assert_eq!(q.rows[0].values(), &[Expr::Now]);
}

#[test]
fn bulk_insert_of_10k_rows_validates() {
    let rows = (0..10_000).map(|i| {
        InsertRow::new()
            .set_value(ID, i)
            .set_value(NAME, format!("u{i}"))
    });
    let q = Insert::into("users").rows(rows);
    assert!(q.validate().is_ok());
    assert_eq!(q.rows.len(), 10_000);
    // 20k parameters: fine on Postgres; 40k rows x 2 cols would pass validate but exceed 65,535.
}

#[test]
fn insert_duplicate_column_in_a_row_is_invalid() {
    let q = Insert::into("users").row(
        InsertRow::new()
            .set_value(NAME, "a".into())
            .set_value(NAME, "b".into()),
    );
    assert!(is_invalid(q.validate()));
}

// --- UPDATE ---

#[test]
fn update_without_assignments_is_invalid_in_every_state() {
    assert!(is_invalid(Update::table("users").validate()));
    assert!(is_invalid(Update::table("users").all().validate()));
    assert!(is_invalid(
        Update::table("users").filter(ID.eq(1)).validate()
    ));
}

#[test]
fn update_filters_are_and_ed_and_set_works_after_filter() {
    let a = ID.eq(1);
    let b = AGE.is_null();
    let q = Update::table("users")
        .filter(a.clone())
        .filter(b.clone())
        .set_value(NAME, "x".into());
    assert_eq!(q.filter, Some(Expr::And(vec![a, b])));
    assert_eq!(q.assignments.len(), 1);
    assert!(q.validate().is_ok());
}

#[test]
fn update_all_has_no_filter() {
    let q = Update::table("users")
        .set(AGE, Expr::value(None::<i32>))
        .all()
        .returning(ID);
    assert_eq!(q.filter, None);
    assert_eq!(q.returning, vec![Expr::from(ID)]);
    assert!(q.validate().is_ok());
}

#[test]
fn update_set_column_from_another_expression() {
    let q = Update::table("users")
        .filter(ID.eq(1))
        .set(NAME, NAME.upper());
    assert_eq!(q.assignments[0].value, NAME.upper());
}

#[test]
fn update_duplicate_assignment_is_invalid() {
    let q = Update::table("users")
        .all()
        .set_value(NAME, "a".into())
        .set_value(NAME, "b".into());
    assert!(is_invalid(q.validate()));
}

// --- DELETE ---

#[test]
fn delete_filters_are_and_ed() {
    let a = ID.eq(1);
    let b = AGE.gt(Some(3));
    let q = Delete::from("users")
        .filter(a.clone())
        .filter(b.clone())
        .returning(ID);
    assert_eq!(q.filter, Some(Expr::And(vec![a, b])));
    assert_eq!(q.returning.len(), 1);
}

#[test]
fn delete_all_has_no_filter() {
    let q = Delete::from("users").all();
    assert_eq!(q.filter, None);
    assert_eq!(q.table, "users");
}

#[test]
fn delete_with_always_true_filter_is_still_filtered() {
    // The typestate can be satisfied with a filter that matches every row.
    // Documented so nobody treats `Filtered` as "safe".
    let q = Delete::from("users").filter(Expr::always_true());
    assert_eq!(q.filter, Some(Expr::And(vec![])));
}

// --- RAW ---

#[test]
fn raw_query_keeps_sql_and_binds_in_order() {
    let q = RawQuery::new("SELECT $1, $2, $3")
        .bind(1_i64)
        .bind_all(["a".to_string(), "b".to_string()]);
    assert_eq!(q.sql, "SELECT $1, $2, $3");
    assert_eq!(q.params.len(), 3);
    assert_eq!(q.params[0], 1_i64.into_value());
}

#[test]
fn raw_query_hostile_param_is_not_spliced_into_sql() {
    let evil = "1; DROP TABLE users; --".to_string();
    let q = RawQuery::new("SELECT * FROM users WHERE name = $1").bind(evil.clone());
    assert!(!q.sql.contains("DROP"));
    assert_eq!(q.params, vec![evil.into_value()]);
}

#[test]
fn raw_query_binds_null() {
    let q = RawQuery::new("SELECT $1").bind(None::<i32>);
    assert_eq!(q.params, vec![Value::Null]);
}
