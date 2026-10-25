//! Unit tests for Now, CountAll and portable Func expressions.

use flux_db_core::ast::expr::{Expr, Func};
use flux_db_core::types::column::TypedColumn;

struct User;
const AGE: TypedColumn<User, Option<i32>> = TypedColumn::new("users", "age");
const NAME: TypedColumn<User, String> = TypedColumn::new("users", "name");

fn call(func: Func, args: Vec<Expr>) -> Expr {
    Expr::Func { func, args }
}

#[test]
fn now_and_count_all_are_const_unit_nodes() {
    const N: Expr = Expr::now();
    const C: Expr = Expr::count_all();
    assert_eq!(N, Expr::Now);
    assert_eq!(C, Expr::CountAll);
    assert_ne!(N, C);
}

#[test]
fn aggregate_shortcuts_wrap_exactly_one_argument() {
    let col = Expr::from(AGE);
    for (built, func) in [
        (AGE.count(), Func::Count),
        (AGE.sum(), Func::Sum),
        (AGE.avg(), Func::Avg),
        (AGE.min(), Func::Min),
        (AGE.max(), Func::Max),
    ] {
        assert_eq!(built, call(func, vec![col.clone()]));
    }
}

#[test]
fn string_shortcuts_wrap_exactly_one_argument() {
    let col = Expr::from(NAME);
    for (built, func) in [
        (NAME.lower(), Func::Lower),
        (NAME.upper(), Func::Upper),
        (NAME.length(), Func::Length),
        (NAME.trim(), Func::Trim),
    ] {
        assert_eq!(built, call(func, vec![col.clone()]));
    }
}

#[test]
fn coalesce_keeps_argument_order() {
    let built = AGE.coalesce(Expr::value(0_i32));
    assert_eq!(
        built,
        call(Func::Coalesce, vec![Expr::from(AGE), Expr::value(0_i32)])
    );
}

#[test]
fn coalesce_can_be_chained_but_nests() {
    // a.coalesce(b).coalesce(c) is COALESCE(COALESCE(a, b), c), not a flat list.
    let built = AGE
        .coalesce(Expr::value(1_i32))
        .coalesce(Expr::value(2_i32));
    let inner = call(Func::Coalesce, vec![Expr::from(AGE), Expr::value(1_i32)]);
    assert_eq!(built, call(Func::Coalesce, vec![inner, Expr::value(2_i32)]));
}

#[test]
fn func_accepts_any_arity_so_renderer_must_validate() {
    // Core does not check argument counts; sql-render returns InvalidQuery (blueprint 01).
    assert_eq!(
        Expr::func(Func::Coalesce, Vec::<Expr>::new()),
        call(Func::Coalesce, vec![])
    );
    let three = Expr::func(
        Func::Count,
        [Expr::from(AGE), Expr::from(AGE), Expr::from(AGE)],
    );
    assert!(matches!(three, Expr::Func { ref args, .. } if args.len() == 3));
}

#[test]
fn nested_functions_compose() {
    let built = NAME.trim().lower().length();
    let expected = call(
        Func::Length,
        vec![call(
            Func::Lower,
            vec![call(Func::Trim, vec![Expr::from(NAME)])],
        )],
    );
    assert_eq!(built, expected);
}

#[test]
fn functions_compare_like_any_expression() {
    let cond = AGE.count().greater_than(Expr::value(5_i64));
    assert!(
        matches!(cond, Expr::Binary { ref left, .. } if matches!(**left, Expr::Func { func: Func::Count, .. }))
    );
    let cond = Expr::count_all().greater_or_equal(Expr::value(1_i64));
    assert!(matches!(cond, Expr::Binary { ref left, .. } if **left == Expr::CountAll));
}

#[test]
fn hostile_text_in_function_args_stays_a_bound_value() {
    let evil = "'); DROP TABLE users; --";
    let built = NAME.coalesce(Expr::value(evil.to_string()));
    let Expr::Func { args, .. } = built else {
        panic!("expected Func")
    };
    assert_eq!(args[1], Expr::value(evil.to_string()));
}

#[test]
fn func_is_copy_and_hashable() {
    use std::collections::HashSet;
    let set: HashSet<Func> = [Func::Sum, Func::Sum, Func::Avg].into_iter().collect();
    assert_eq!(set.len(), 2);
    let f = Func::Max;
    let g = f;
    assert_eq!(f, g);
}
