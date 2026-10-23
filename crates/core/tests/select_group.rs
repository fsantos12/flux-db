//! Unit tests for GROUP BY / HAVING on `Select`.

use flux_db_core::ast::expr::Expr;
use flux_db_core::ast::group::GroupBy;
use flux_db_core::query::select::Select;
use flux_db_core::query::{IntoQuery, Query};
use flux_db_core::types::column::TypedColumn;

struct User;
const ID: TypedColumn<User, i64> = TypedColumn::new("users", "id");
const CITY: TypedColumn<User, String> = TypedColumn::new("users", "city");
const AGE: TypedColumn<User, i32> = TypedColumn::new("users", "age");

fn count_gt_one() -> Expr {
    ID.count().greater_than(Expr::value(1i64))
}

fn group(q: &Select) -> &GroupBy {
    q.group.as_ref().expect("group should be set")
}

#[test]
fn select_without_group_by_has_no_group() {
    assert_eq!(Select::from("users").group, None);
}

#[test]
fn group_by_keeps_expression_order() {
    let q = Select::from("users").group_by([Expr::from(CITY), Expr::from(AGE)]);
    assert_eq!(group(&q).exprs, vec![Expr::from(CITY), Expr::from(AGE)]);
    assert_eq!(group(&q).having, None);
}

#[test]
fn repeated_group_by_calls_append() {
    let q = Select::from("users").group_by([CITY]).group_by([AGE]);
    assert_eq!(group(&q).exprs, vec![Expr::from(CITY), Expr::from(AGE)]);
}

#[test]
fn group_by_accepts_function_expressions() {
    let q = Select::from("users").group_by([CITY.lower()]);
    assert_eq!(group(&q).exprs, vec![CITY.lower()]);
}

#[test]
fn single_having_is_stored_as_is() {
    let q = Select::from("users")
        .group_by([CITY])
        .having(count_gt_one());
    assert_eq!(group(&q).having, Some(count_gt_one()));
}

#[test]
fn repeated_having_calls_are_and_ed() {
    let b = AGE.max().less_than(Expr::value(90));
    let q = Select::from("users")
        .group_by([CITY])
        .having(count_gt_one())
        .having(b.clone());
    assert_eq!(group(&q).having, Some(Expr::And(vec![count_gt_one(), b])));
}

#[test]
fn having_and_where_are_independent() {
    let w = AGE.ge(18);
    let q = Select::from("users")
        .filter(w.clone())
        .group_by([CITY])
        .having(count_gt_one());
    assert_eq!(q.filter, Some(w));
    assert_eq!(group(&q).having, Some(count_gt_one()));
}

#[test]
fn builder_order_does_not_matter() {
    let a = Select::from("users")
        .group_by([CITY])
        .having(count_gt_one())
        .column(CITY);
    let b = Select::from("users")
        .column(CITY)
        .having(count_gt_one())
        .group_by([CITY]);
    assert_eq!(a, b);
}

#[test]
fn group_by_after_having_keeps_the_having() {
    let q = Select::from("users")
        .having(count_gt_one())
        .group_by([Expr::from(CITY), Expr::from(AGE)]);
    assert_eq!(
        q.group,
        Some(GroupBy::new([Expr::from(CITY), Expr::from(AGE)]).having(count_gt_one()))
    );
}

#[test]
fn group_by_matches_ast_group_node() {
    let q = Select::from("users")
        .group_by([CITY])
        .having(count_gt_one());
    assert_eq!(q.group, Some(GroupBy::new([CITY]).having(count_gt_one())));
}

#[test]
fn grouped_select_converts_to_query_unchanged() {
    let q = Select::from("users")
        .column(CITY)
        .column(Expr::count_all())
        .group_by([CITY])
        .having(count_gt_one());
    assert_eq!(q.clone().into_query().unwrap(), Query::Select(q));
}

// --- Current behavior, left to the renderer (sql-render) to accept or reject ---

#[test]
fn having_without_group_by_is_kept_with_empty_exprs() {
    let q = Select::from("users").having(count_gt_one());
    let g = group(&q);
    assert!(g.is_empty());
    assert_eq!(g.having, Some(count_gt_one()));
}

#[test]
fn empty_group_by_list_sets_an_empty_group() {
    let q = Select::from("users").group_by(Vec::<Expr>::new());
    assert!(group(&q).is_empty());
    assert_ne!(q, Select::from("users"));
}

// --- Stress ---

#[test]
fn many_group_by_terms_and_having_conditions() {
    let mut q = Select::from("users");
    for _ in 0..1_000 {
        q = q.group_by([CITY]).having(count_gt_one());
    }
    let g = group(&q);
    assert_eq!(g.exprs.len(), 1_000);
    // `Expr::and` flattens, so 1,000 HAVING calls stay one flat AND.
    assert!(matches!(&g.having, Some(Expr::And(parts)) if parts.len() == 1_000));
}
