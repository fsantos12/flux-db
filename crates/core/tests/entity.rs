//! Entity trait: column metadata, primary key values and `primary_key_filter`.

use flux_db_core::ast::expr::Expr;
use flux_db_core::entity::{Entity, EntityColumnDef};
use flux_db_core::types::{
    column::Column,
    error::{Error, Result},
    row::{FromRow, Row},
    value::{IntoValue, Value},
};

const ORG: EntityColumnDef = EntityColumnDef {
    name: "org_id",
    primary_key: true,
};
const USER: EntityColumnDef = EntityColumnDef {
    name: "user_id",
    primary_key: true,
};
const ROLE: EntityColumnDef = EntityColumnDef {
    name: "role",
    primary_key: false,
};

/// Composite key: (org_id, user_id).
struct Membership {
    org_id: i64,
    user_id: i32,
    role: String,
}

impl FromRow for Membership {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            org_id: row.get("org_id")?,
            user_id: row.get("user_id")?,
            role: row.get("role")?,
        })
    }
}

impl Entity for Membership {
    const TABLE: &'static str = "memberships";
    const PRIMARY_KEY: &'static [EntityColumnDef] = &[ORG, USER];
    const COLUMNS: &'static [EntityColumnDef] = &[ORG, USER, ROLE];

    fn primary_key_values(&self) -> Vec<(Column, Value)> {
        vec![
            (ORG.column::<Self>(), self.org_id.into_value()),
            (USER.column::<Self>(), self.user_id.into_value()),
        ]
    }

    fn column_values(&self) -> Vec<(Column, Value)> {
        vec![(ROLE.column::<Self>(), self.role.clone().into_value())]
    }
}

/// Single key.
struct User;
impl FromRow for User {
    fn from_row(_: &Row) -> Result<Self> {
        Ok(Self)
    }
}
impl Entity for User {
    const TABLE: &'static str = "users";
    const PRIMARY_KEY: &'static [EntityColumnDef] = &[EntityColumnDef {
        name: "id",
        primary_key: true,
    }];
    const COLUMNS: &'static [EntityColumnDef] = Self::PRIMARY_KEY;
    fn column_values(&self) -> Vec<(Column, Value)> {
        Vec::new()
    }
    fn primary_key_values(&self) -> Vec<(Column, Value)> {
        vec![(Column::qualified("users", "id"), 1_i64.into_value())]
    }
}

/// No key.
struct LogLine;
impl FromRow for LogLine {
    fn from_row(_: &Row) -> Result<Self> {
        Ok(Self)
    }
}
impl Entity for LogLine {
    const TABLE: &'static str = "log";
    const PRIMARY_KEY: &'static [EntityColumnDef] = &[];
    const COLUMNS: &'static [EntityColumnDef] = &[EntityColumnDef {
        name: "msg",
        primary_key: false,
    }];
    fn column_values(&self) -> Vec<(Column, Value)> {
        Vec::new()
    }
    fn primary_key_values(&self) -> Vec<(Column, Value)> {
        Vec::new()
    }
}

/// Wide key with no fixed limit on key columns.
const WIDE_KEY: &[EntityColumnDef] = &[
    EntityColumnDef {
        name: "a",
        primary_key: true,
    },
    EntityColumnDef {
        name: "b",
        primary_key: true,
    },
    EntityColumnDef {
        name: "c",
        primary_key: true,
    },
    EntityColumnDef {
        name: "d",
        primary_key: true,
    },
    EntityColumnDef {
        name: "e",
        primary_key: true,
    },
    EntityColumnDef {
        name: "f",
        primary_key: true,
    },
];
struct Wide;
impl FromRow for Wide {
    fn from_row(_: &Row) -> Result<Self> {
        Ok(Self)
    }
}
impl Entity for Wide {
    const TABLE: &'static str = "wide";
    const PRIMARY_KEY: &'static [EntityColumnDef] = WIDE_KEY;
    const COLUMNS: &'static [EntityColumnDef] = WIDE_KEY;
    fn column_values(&self) -> Vec<(Column, Value)> {
        Vec::new()
    }
    fn primary_key_values(&self) -> Vec<(Column, Value)> {
        WIDE_KEY
            .iter()
            .zip(0..6_i32)
            .map(|(d, v)| (d.column::<Self>(), v.into_value()))
            .collect()
    }
}

/// Bare key values, as `find` would receive them.
fn bare(e: &impl Entity) -> Vec<Value> {
    e.primary_key_values().into_iter().map(|(_, v)| v).collect()
}

fn sample() -> Membership {
    Membership {
        org_id: 7,
        user_id: 42,
        role: "admin".into(),
    }
}

fn eq(table: &'static str, col: &'static str, v: impl IntoValue) -> Expr {
    Expr::from(Column::qualified(table, col)).equals(v.into_value())
}

fn assert_invalid(r: Result<Expr>, needle: &str) {
    match r {
        Err(Error::InvalidQuery(msg)) => assert!(msg.contains(needle), "unexpected message: {msg}"),
        other => panic!("expected InvalidQuery containing {needle:?}, got {other:?}"),
    }
}

// Metadata

#[test]
fn column_def_is_qualified_by_entity_table() {
    assert_eq!(
        ORG.column::<Membership>(),
        Column::qualified("memberships", "org_id")
    );
    assert_eq!(ROLE.column::<LogLine>(), Column::qualified("log", "role"));
}

#[test]
fn primary_key_matches_flagged_columns_in_order() {
    let flagged: Vec<_> = Membership::COLUMNS
        .iter()
        .filter(|c| c.primary_key)
        .copied()
        .collect();
    assert_eq!(flagged, Membership::PRIMARY_KEY);
}

#[test]
fn values_cover_every_column_in_field_order() {
    let cols: Vec<_> = sample().values().into_iter().map(|(c, _)| c).collect();
    let expected: Vec<_> = Membership::COLUMNS
        .iter()
        .map(|c| c.column::<Membership>())
        .collect();
    assert_eq!(cols, expected);
}

#[test]
fn primary_key_values_follow_primary_key_order() {
    let v = bare(&sample());
    assert_eq!(v, vec![7_i64.into_value(), 42_i32.into_value()]);
    let cols: Vec<_> = sample()
        .primary_key_values()
        .into_iter()
        .map(|(c, _)| c)
        .collect();
    let expected: Vec<_> = Membership::PRIMARY_KEY
        .iter()
        .map(|d| d.column::<Membership>())
        .collect();
    assert_eq!(cols, expected);
    assert_eq!(v.len(), Membership::PRIMARY_KEY.len());
    assert!(LogLine.primary_key_values().is_empty());
}

#[test]
fn row_round_trip_through_from_row() {
    let row = Row::new(
        ["org_id", "user_id", "role"].map(Box::<str>::from).into(),
        vec![
            7_i64.into_value(),
            42_i32.into_value(),
            "admin".into_value(),
        ],
    );
    let m = Membership::from_row(&row).unwrap();
    assert_eq!((m.org_id, m.user_id, m.role.as_str()), (7, 42, "admin"));
}

#[test]
fn column_values_exclude_every_key_column() {
    let set: Vec<_> = sample()
        .column_values()
        .into_iter()
        .map(|(c, _)| c)
        .collect();
    assert_eq!(set, vec![ROLE.column::<Membership>()]);
    for key in Membership::PRIMARY_KEY {
        assert!(
            !set.contains(&key.column::<Membership>()),
            "key `{}` in SET",
            key.name
        );
    }
    assert!(User.column_values().is_empty());
}

#[test]
fn values_are_keys_then_column_values() {
    let mut expected = sample().primary_key_values();
    expected.extend(sample().column_values());
    assert_eq!(sample().values(), expected);
    assert_eq!(LogLine.values(), LogLine.column_values());
}

// own_key_filter

#[test]
fn own_key_filter_matches_primary_key_filter() {
    for (own, bare_f) in [
        (
            sample().own_key_filter(),
            Membership::primary_key_filter(bare(&sample())),
        ),
        (User.own_key_filter(), User::primary_key_filter(bare(&User))),
        (Wide.own_key_filter(), Wide::primary_key_filter(bare(&Wide))),
    ] {
        assert_eq!(own.unwrap(), bare_f.unwrap());
    }
}

#[test]
fn own_key_filter_without_key_is_invalid() {
    assert_invalid(LogLine.own_key_filter(), "no primary key");
}

#[test]
fn own_key_filter_rejects_null_key_part() {
    struct Draft(Option<i64>);
    impl FromRow for Draft {
        fn from_row(_: &Row) -> Result<Self> {
            Ok(Self(None))
        }
    }
    impl Entity for Draft {
        const TABLE: &'static str = "drafts";
        const PRIMARY_KEY: &'static [EntityColumnDef] = &[ORG, USER];
        const COLUMNS: &'static [EntityColumnDef] = &[ORG, USER];
        fn primary_key_values(&self) -> Vec<(Column, Value)> {
            vec![
                (ORG.column::<Self>(), 1_i64.into_value()),
                (USER.column::<Self>(), self.0.into_value()),
            ]
        }
        fn column_values(&self) -> Vec<(Column, Value)> {
            Vec::new()
        }
    }
    assert_invalid(Draft(None).own_key_filter(), "`user_id` cannot be NULL");
    assert!(Draft(Some(2)).own_key_filter().is_ok());
}

// primary_key_filter: happy paths

#[test]
fn single_key_filter() {
    let f = User::primary_key_filter(bare(&User)).unwrap();
    assert_eq!(f, Expr::all([eq("users", "id", 1_i64)]));
}

#[test]
fn composite_filter_ands_in_primary_key_order() {
    let f = Membership::primary_key_filter(bare(&sample())).unwrap();
    assert_eq!(
        f,
        Expr::all([
            eq("memberships", "org_id", 7_i64),
            eq("memberships", "user_id", 42_i32)
        ])
    );
    // Swapped values bind to the same columns, so the filter changes.
    let swapped =
        Membership::primary_key_filter(vec![42_i32.into_value(), 7_i64.into_value()]).unwrap();
    assert_ne!(f, swapped);
}

#[test]
fn filter_has_no_column_limit() {
    let f = Wide::primary_key_filter(bare(&Wide)).unwrap();
    let expected = Expr::all(
        WIDE_KEY
            .iter()
            .zip(0..6_i32)
            .map(|(d, v)| eq("wide", d.name, v)),
    );
    assert_eq!(f, expected);
}

#[test]
fn filter_is_never_always_true() {
    // An empty AND matches every row; a valid key must never produce one.
    for f in [
        User::primary_key_filter(bare(&User)).unwrap(),
        Membership::primary_key_filter(bare(&sample())).unwrap(),
        Wide::primary_key_filter(bare(&Wide)).unwrap(),
    ] {
        assert_ne!(f, Expr::always_true());
    }
}

// primary_key_filter: InvalidQuery cases

#[test]
fn no_primary_key_is_invalid() {
    assert_invalid(LogLine::primary_key_filter(Vec::new()), "no primary key");
    assert_invalid(
        LogLine::primary_key_filter(vec![1.into_value()]),
        "no primary key",
    );
}

#[test]
fn wrong_value_count_is_invalid() {
    assert_invalid(Membership::primary_key_filter(Vec::new()), "expects 2");
    assert_invalid(
        Membership::primary_key_filter(vec![1.into_value()]),
        "got 1",
    );
    assert_invalid(
        Membership::primary_key_filter(vec![1.into_value(); 3]),
        "got 3",
    );
    assert_invalid(User::primary_key_filter(Vec::new()), "got 0");
}

#[test]
fn null_value_is_invalid_in_any_position() {
    assert_invalid(
        User::primary_key_filter(vec![Value::Null]),
        "`id` cannot be NULL",
    );
    assert_invalid(
        Membership::primary_key_filter(vec![Value::Null, 1.into_value()]),
        "`org_id`",
    );
    assert_invalid(
        Membership::primary_key_filter(vec![1.into_value(), Value::Null]),
        "`user_id`",
    );
    assert_invalid(
        Membership::primary_key_filter(None::<i64>.into_values_vec()),
        "NULL",
    );
}

trait IntoVec {
    fn into_values_vec(self) -> Vec<Value>;
}
impl IntoVec for Option<i64> {
    /// `None` through `IntoValue` must still be rejected, padded to the key width.
    fn into_values_vec(self) -> Vec<Value> {
        vec![self.into_value(), self.into_value()]
    }
}

// Stress

#[test]
fn stress_many_filters() {
    for i in 0..10_000_i64 {
        let m = Membership {
            org_id: i,
            user_id: -(i as i32),
            role: String::new(),
        };
        let f = m.own_key_filter().unwrap();
        assert_eq!(
            f,
            Expr::all([
                eq("memberships", "org_id", i),
                eq("memberships", "user_id", -(i as i32))
            ])
        );
    }
}

#[test]
fn stress_wide_key() {
    struct Huge;
    impl FromRow for Huge {
        fn from_row(_: &Row) -> Result<Self> {
            Ok(Self)
        }
    }
    const N: usize = 64;
    const KEY: [EntityColumnDef; N] = [EntityColumnDef {
        name: "k",
        primary_key: true,
    }; N];
    impl Entity for Huge {
        const TABLE: &'static str = "huge";
        const PRIMARY_KEY: &'static [EntityColumnDef] = &KEY;
        const COLUMNS: &'static [EntityColumnDef] = &KEY;
        fn column_values(&self) -> Vec<(Column, Value)> {
            Vec::new()
        }
        fn primary_key_values(&self) -> Vec<(Column, Value)> {
            KEY.iter()
                .zip(0..N as i64)
                .map(|(d, v)| (d.column::<Self>(), v.into_value()))
                .collect()
        }
    }
    let Expr::And(parts) = Huge::primary_key_filter(bare(&Huge)).unwrap() else {
        panic!("expected And");
    };
    assert_eq!(parts.len(), N);
}

#[test]
fn values_have_no_missing_or_duplicate_columns() {
    let cols: Vec<_> = sample().values().into_iter().map(|(c, _)| c).collect();
    let mut deduped = cols.clone();
    deduped.dedup_by(|a, b| a == b);
    assert_eq!(cols.len(), deduped.len(), "duplicate column in values()");
    assert_eq!(cols.len(), Membership::COLUMNS.len());
    for def in Membership::COLUMNS {
        assert!(
            cols.contains(&def.column::<Membership>()),
            "`{}` missing",
            def.name
        );
    }
}

#[test]
fn key_only_entity_has_empty_set_list() {
    // `update` must turn this into InvalidQuery, never `UPDATE t SET WHERE ...`.
    assert!(User.column_values().is_empty());
    assert!(User.own_key_filter().is_ok());
}
