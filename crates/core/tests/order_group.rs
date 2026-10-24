//! Unit tests for ORDER BY and GROUP BY / HAVING nodes.

use flux_db_core::ast::expr::Expr;
use flux_db_core::ast::group::GroupBy;
use flux_db_core::ast::order::{Direction, NullsOrder, OrderBy};
use flux_db_core::types::column::{Column, TypedColumn};

struct User;
const AGE: TypedColumn<User, Option<i32>> = TypedColumn::new("users", "age");
const NAME: TypedColumn<User, String> = TypedColumn::new("users", "name");

#[test]
fn defaults_are_asc_and_database_nulls_order() {
    assert_eq!(Direction::default(), Direction::Asc);
    assert_eq!(NullsOrder::default(), NullsOrder::Default);
}

#[test]
fn typed_column_asc_desc_build_order_items() {
    assert_eq!(
        AGE.asc(),
        OrderBy {
            expr: Expr::from(AGE),
            direction: Direction::Asc,
            nulls: NullsOrder::Default
        }
    );
    assert_eq!(
        AGE.desc(),
        OrderBy {
            expr: Expr::from(AGE),
            direction: Direction::Desc,
            nulls: NullsOrder::Default
        }
    );
}

#[test]
fn nulls_first_and_last_keep_direction() {
    let o = AGE.desc().nulls_first();
    assert_eq!((o.direction, o.nulls), (Direction::Desc, NullsOrder::First));
    let o = AGE.asc().nulls_last();
    assert_eq!((o.direction, o.nulls), (Direction::Asc, NullsOrder::Last));
}

#[test]
fn last_nulls_call_wins() {
    assert_eq!(AGE.asc().nulls_first().nulls_last().nulls, NullsOrder::Last);
}

#[test]
fn order_by_accepts_untyped_columns_and_expressions() {
    let o = OrderBy::desc(Column::new("created_at"));
    assert_eq!(o.expr, Expr::Column(Column::new("created_at")));
    let o = OrderBy::asc(Expr::raw("lower(name)", vec![]));
    assert!(matches!(o.expr, Expr::Raw { .. }));
}

#[test]
fn group_by_keeps_column_order() {
    let g = GroupBy::new([Column::new("b"), Column::new("a")]);
    assert_eq!(
        g.exprs,
        vec![
            Expr::Column(Column::new("b")),
            Expr::Column(Column::new("a"))
        ]
    );
    assert_eq!(g.having, None);
    assert!(!g.is_empty());
}

#[test]
fn group_by_typed_columns_of_different_types() {
    let g = GroupBy::new([Expr::from(AGE), Expr::from(NAME)]);
    assert_eq!(g.exprs.len(), 2);
}

#[test]
fn empty_group_by_is_empty() {
    assert!(GroupBy::default().is_empty());
    assert!(GroupBy::new(Vec::<Expr>::new()).is_empty());
}

#[test]
fn single_having_is_stored_as_is() {
    let cond = AGE.gt(Some(18));
    assert_eq!(GroupBy::new([AGE]).having(cond.clone()).having, Some(cond));
}

#[test]
fn repeated_having_is_and_ed() {
    let a = AGE.gt(Some(18));
    let b = AGE.lt(Some(65));
    let c = NAME.ne("x".into());
    let g = GroupBy::new([AGE])
        .having(a.clone())
        .having(b.clone())
        .having(c.clone());
    assert_eq!(g.having, Some(Expr::And(vec![a, b, c])));
}

#[test]
fn having_after_an_or_keeps_the_or_grouped() {
    let a = AGE.eq(Some(1));
    let b = AGE.eq(Some(2));
    let c = AGE.eq(Some(3));
    let g = GroupBy::new([AGE])
        .having(a.clone() | b.clone())
        .having(c.clone());
    assert_eq!(g.having, Some(Expr::And(vec![Expr::Or(vec![a, b]), c])));
}

#[test]
fn having_without_group_columns_is_still_kept() {
    // `is_empty()` only looks at exprs: a HAVING with no GROUP BY must not be silently dropped
    // by the renderer (it should render HAVING alone or return InvalidQuery).
    let g = GroupBy::default().having(AGE.gt(Some(1)));
    assert!(g.is_empty());
    assert!(g.having.is_some());
}
