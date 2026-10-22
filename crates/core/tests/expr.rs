//! Unit tests for the `Expr` AST and typed column helpers.

use flux_db_core::ast::expr::{BinaryOp, Expr, LikePattern};
use flux_db_core::types::column::{Column, TypedColumn};
use flux_db_core::types::value::Value;

struct User;

const ID: TypedColumn<User, i64> = TypedColumn::new("users", "id");
const NAME: TypedColumn<User, String> = TypedColumn::new("users", "name");
const EMAIL: TypedColumn<User, Option<String>> = TypedColumn::new("users", "email");
const AGE: TypedColumn<User, Option<i32>> = TypedColumn::new("users", "age");

fn col(c: Column) -> Box<Expr> {
    Box::new(Expr::Column(c))
}
fn val(v: Value) -> Box<Expr> {
    Box::new(Expr::Value(v))
}

#[test]
fn typed_column_converts_to_qualified_column_expr() {
    assert_eq!(
        Expr::from(ID),
        Expr::Column(Column::qualified("users", "id"))
    );
    assert_eq!(Expr::col(ID), Expr::from(ID));
    assert_eq!(Expr::col(Column::new("x")), Expr::Column(Column::new("x")));
}

#[test]
fn dotted_name_is_one_identifier_not_split() {
    let c = Column::new("users.age");
    assert_eq!(c.name(), "users.age");
    assert_eq!(c.table(), None);
}

#[test]
fn typed_comparisons_build_binary_nodes_with_bound_values() {
    let cases = [
        (ID.eq(1), BinaryOp::Eq),
        (ID.ne(1), BinaryOp::Ne),
        (ID.lt(1), BinaryOp::Lt),
        (ID.le(1), BinaryOp::Le),
        (ID.gt(1), BinaryOp::Gt),
        (ID.ge(1), BinaryOp::Ge),
    ];
    for (expr, op) in cases {
        assert_eq!(
            expr,
            Expr::Binary {
                left: col(ID.column()),
                op,
                right: val(Value::I64(1))
            }
        );
    }
}

#[test]
fn hostile_text_stays_a_value_never_sql() {
    let evil = "x'; DROP TABLE users; --";
    assert_eq!(
        NAME.eq(evil.to_owned()),
        Expr::Binary {
            left: col(NAME.column()),
            op: BinaryOp::Eq,
            right: val(Value::String(evil.to_owned())),
        }
    );
}

#[test]
fn eq_none_builds_is_null_and_ne_none_builds_is_not_null() {
    assert_eq!(AGE.eq(None), Expr::IsNull(col(AGE.column())));
    assert_eq!(AGE.ne(None), Expr::IsNotNull(col(AGE.column())));
}

#[test]
fn eq_some_on_nullable_column_binds_the_inner_value() {
    assert_eq!(
        AGE.eq(Some(30)),
        Expr::Binary {
            left: col(AGE.column()),
            op: BinaryOp::Eq,
            right: val(Value::I32(30))
        }
    );
}

#[test]
fn is_null_and_is_not_null_on_option_columns() {
    assert_eq!(EMAIL.is_null(), Expr::IsNull(col(EMAIL.column())));
    assert_eq!(EMAIL.is_not_null(), Expr::IsNotNull(col(EMAIL.column())));
}

#[test]
fn in_lists_keep_order_and_values() {
    assert_eq!(
        ID.is_in([3, 1, 2]),
        Expr::In {
            expr: col(ID.column()),
            list: vec![Expr::value(3i64), Expr::value(1i64), Expr::value(2i64)],
        }
    );
    assert_eq!(
        ID.not_in(vec![1]),
        Expr::NotIn {
            expr: col(ID.column()),
            list: vec![Expr::value(1i64)]
        }
    );
}

#[test]
fn empty_in_lists_are_kept_empty_for_the_renderer() {
    assert_eq!(
        ID.is_in([]),
        Expr::In {
            expr: col(ID.column()),
            list: vec![]
        }
    );
    assert_eq!(
        ID.not_in([]),
        Expr::NotIn {
            expr: col(ID.column()),
            list: vec![]
        }
    );
}

#[test]
fn not_in_with_null_is_represented_as_null_value() {
    // SQL: `x NOT IN (1, NULL)` returns no rows. Renderer/driver must handle or document it.
    assert_eq!(
        AGE.not_in([Some(1), None]),
        Expr::NotIn {
            expr: col(AGE.column()),
            list: vec![Expr::Value(Value::I32(1)), Expr::Value(Value::Null)],
        }
    );
}

#[test]
fn very_large_in_list_is_built_without_loss() {
    let n = 70_000; // above Postgres' 65535 bind limit; renderer must reject it.
    match ID.is_in(0..n) {
        Expr::In { list, .. } => assert_eq!(list.len(), n as usize),
        other => panic!("expected In, got {other:?}"),
    }
}

#[test]
fn between_and_not_between_are_inclusive_nodes() {
    assert_eq!(
        ID.between(1, 10),
        Expr::Between {
            expr: col(ID.column()),
            low: val(Value::I64(1)),
            high: val(Value::I64(10))
        }
    );
    assert_eq!(
        ID.not_between(10, 1),
        Expr::NotBetween {
            expr: col(ID.column()),
            low: val(Value::I64(10)),
            high: val(Value::I64(1))
        }
    );
}

#[test]
fn like_helpers_keep_text_unescaped_for_the_dialect() {
    let cases = [
        (NAME.contains("50%"), LikePattern::Contains("50%".into())),
        (
            NAME.starts_with("a_b"),
            LikePattern::StartsWith("a_b".into()),
        ),
        (NAME.ends_with("\\"), LikePattern::EndsWith("\\".into())),
        (NAME.contains(""), LikePattern::Contains(String::new())),
        (NAME.like("%%__"), LikePattern::Raw("%%__".into())),
    ];
    for (expr, pattern) in cases {
        assert_eq!(
            expr,
            Expr::Like {
                expr: col(NAME.column()),
                pattern
            }
        );
    }
    assert_eq!(
        NAME.not_like("a%"),
        Expr::NotLike {
            expr: col(NAME.column()),
            pattern: LikePattern::Raw("a%".into())
        }
    );
}

#[test]
fn and_or_flatten_on_the_left() {
    let a = ID.eq(1);
    let b = ID.eq(2);
    let c = ID.eq(3);
    assert_eq!(
        a.clone().and(b.clone()).and(c.clone()),
        Expr::And(vec![a.clone(), b.clone(), c.clone()])
    );
    assert_eq!(
        a.clone().or(b.clone()).or(c.clone()),
        Expr::Or(vec![a.clone(), b.clone(), c.clone()])
    );
    assert_eq!(a.clone() & b.clone(), Expr::And(vec![a.clone(), b.clone()]));
    assert_eq!(a.clone() | b.clone(), Expr::Or(vec![a.clone(), b.clone()]));
}

#[test]
fn and_does_not_merge_an_or_and_vice_versa() {
    let a = ID.eq(1);
    let b = ID.eq(2);
    let c = ID.eq(3);
    // (a OR b) AND c must keep the OR as one group.
    assert_eq!(
        (a.clone() | b.clone()) & c.clone(),
        Expr::And(vec![Expr::Or(vec![a, b]), c])
    );
}

#[test]
fn not_wraps_the_whole_expression() {
    let e = ID.eq(1) & ID.eq(2);
    assert_eq!(!e.clone(), Expr::Not(Box::new(e)));
}

#[test]
fn empty_all_and_any_are_always_true_and_false() {
    assert_eq!(Expr::all(Vec::new()), Expr::always_true());
    assert_eq!(Expr::any(Vec::new()), Expr::always_false());
    assert_eq!(Expr::always_true(), Expr::And(vec![]));
    assert_eq!(Expr::always_false(), Expr::Or(vec![]));
}

#[test]
fn chaining_onto_always_true_keeps_semantics() {
    let a = ID.eq(1);
    assert_eq!(
        Expr::always_true().and(a.clone()),
        Expr::And(vec![a.clone()])
    );
    assert_eq!(Expr::always_false().or(a.clone()), Expr::Or(vec![a]));
}

#[test]
fn untyped_comparisons_accept_any_expression() {
    let e = Expr::col(ID).greater_or_equal(Expr::col(Column::new("min_id")));
    assert_eq!(
        e,
        Expr::Binary {
            left: col(ID.column()),
            op: BinaryOp::Ge,
            right: Box::new(Expr::Column(Column::new("min_id"))),
        }
    );
    assert_eq!(
        Expr::col(ID).in_list([Value::I64(1)]),
        Expr::In {
            expr: col(ID.column()),
            list: vec![Expr::Value(Value::I64(1))]
        }
    );
}

#[test]
fn raw_keeps_params_separate_from_sql() {
    let e = Expr::raw("lower(name) = ?", vec![Value::String("x'--".into())]);
    assert_eq!(
        e,
        Expr::Raw {
            sql: "lower(name) = ?".into(),
            params: vec![Value::String("x'--".into())]
        }
    );
}

#[test]
fn deeply_nested_expression_builds_and_clones() {
    // Derived Clone/PartialEq recurse; ~2000 levels overflows a 2 MB test stack in debug.
    // The renderer should reject very deep trees with InvalidQuery instead of crashing.
    let mut e = ID.eq(0);
    for i in 1..200 {
        e = !(e | ID.eq(i));
    }
    let copy = e.clone();
    assert_eq!(copy, e);
}

#[test]
fn deeply_nested_expression_can_be_dropped() {
    let mut e = ID.eq(0);
    for i in 1..2_000 {
        e = !(e | ID.eq(i));
    }
    drop(e);
}
