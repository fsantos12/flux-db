//! Executor shortcuts on the builders: each one must send exactly the `Query`
//! that `into_query()` builds, to the matching `Executor` method, and decode
//! the result through `FromRow`.

use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};

use flux_db_core::ast::expr::Expr;
use flux_db_core::driver::Executor;
use flux_db_core::query::{
    IntoQuery, Query, delete::Delete, insert::Insert, insert::InsertRow, raw::RawQuery,
    select::Select, update::Update,
};
use flux_db_core::types::{
    column::Column,
    error::{Error, Result},
    row::{FromRow, Row},
    value::{IntoValue, Value},
};
use futures_core::Stream;

/// Polls a future that the mock always completes immediately.
fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = pin!(f);
    match f.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(v) => v,
        Poll::Pending => panic!("mock future should be ready"),
    }
}

struct NoRows;
impl Stream for NoRows {
    type Item = Result<Row>;
    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(None)
    }
}

/// Which `Executor` method received a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Call {
    Execute,
    FetchAll,
    FetchOneOptional,
}

/// Records every call; returns canned rows and an affected-row count.
#[derive(Default)]
struct Mock {
    calls: Mutex<Vec<(Call, Query)>>,
    rows: Vec<Row>,
    affected: u64,
    fail: bool,
}

impl Mock {
    fn rows(rows: Vec<Row>) -> Self {
        Self {
            rows,
            ..Self::default()
        }
    }
    fn affecting(n: u64) -> Self {
        Self {
            affected: n,
            ..Self::default()
        }
    }
    fn failing() -> Self {
        Self {
            fail: true,
            ..Self::default()
        }
    }
    fn record(&self, call: Call, q: &Query) -> Result<()> {
        self.calls.lock().unwrap().push((call, q.clone()));
        if self.fail {
            Err(Error::driver("boom"))
        } else {
            Ok(())
        }
    }
    fn calls(&self) -> Vec<(Call, Query)> {
        self.calls.lock().unwrap().clone()
    }
}

impl Executor for Mock {
    fn execute(&self, q: &Query) -> impl Future<Output = Result<u64>> + Send {
        let r = self.record(Call::Execute, q).map(|()| self.affected);
        async move { r }
    }
    fn fetch<'a>(&'a self, _: &'a Query) -> impl Stream<Item = Result<Row>> + Send + 'a {
        NoRows
    }
    fn fetch_all(&self, q: &Query) -> impl Future<Output = Result<Vec<Row>>> + Send {
        let r = self.record(Call::FetchAll, q).map(|()| self.rows.clone());
        async move { r }
    }
    fn fetch_one_optional(&self, q: &Query) -> impl Future<Output = Result<Option<Row>>> + Send {
        let r = self
            .record(Call::FetchOneOptional, q)
            .map(|()| self.rows.first().cloned());
        async move { r }
    }
}

#[derive(Debug, PartialEq)]
struct User {
    id: i64,
    name: String,
}

impl FromRow for User {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
        })
    }
}

fn row(id: i64, name: &str) -> Row {
    Row::new(
        ["id", "name"].map(Box::<str>::from).into(),
        vec![id.into_value(), name.into_value()],
    )
}

fn users() -> Vec<Row> {
    vec![row(1, "ann"), row(2, "bob")]
}

fn id_is(id: i64) -> Expr {
    Expr::from(Column::new("id")).equals(id.into_value())
}

fn select() -> Select {
    Select::from("users")
        .columns([Column::new("id"), Column::new("name")])
        .filter(id_is(1))
}

fn insert() -> Insert {
    Insert::into("users").row(InsertRow::new().set(Column::new("name"), "ann".into_value()))
}

fn assert_invalid<T: std::fmt::Debug>(r: Result<T>) {
    assert!(
        matches!(r, Err(Error::InvalidQuery(_))),
        "expected InvalidQuery, got {r:?}"
    );
}

// Select

#[test]
fn select_fetch_all_sends_same_query_and_decodes() {
    let db = Mock::rows(users());
    let got: Vec<User> = block_on(select().fetch_all(&db)).unwrap();
    assert_eq!(
        got,
        vec![
            User {
                id: 1,
                name: "ann".into()
            },
            User {
                id: 2,
                name: "bob".into()
            }
        ]
    );
    assert_eq!(
        db.calls(),
        vec![(Call::FetchAll, select().into_query().unwrap())]
    );
}

#[test]
fn select_fetch_one_returns_first_row() {
    let db = Mock::rows(users());
    let got: User = block_on(select().fetch_one(&db)).unwrap();
    assert_eq!(got.id, 1);
    // fetch_one goes through fetch_one_optional (the Executor default).
    assert_eq!(
        db.calls(),
        vec![(Call::FetchOneOptional, select().into_query().unwrap())]
    );
}

#[test]
fn select_fetch_one_on_empty_is_not_found() {
    let r = block_on(select().fetch_one::<User>(&Mock::default()));
    assert!(matches!(r, Err(Error::NotFound)), "{r:?}");
}

#[test]
fn select_fetch_one_optional_some_and_none() {
    let some = block_on(select().fetch_one_optional::<User>(&Mock::rows(users()))).unwrap();
    assert_eq!(
        some,
        Some(User {
            id: 1,
            name: "ann".into()
        })
    );
    let none = block_on(select().fetch_one_optional::<User>(&Mock::default())).unwrap();
    assert_eq!(none, None);
}

#[test]
fn select_fetch_all_empty_is_empty_vec() {
    assert!(
        block_on(select().fetch_all::<User>(&Mock::default()))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn untyped_rows_via_row_from_row() {
    let got: Vec<Row> = block_on(select().fetch_all(&Mock::rows(users()))).unwrap();
    assert_eq!(got, users());
    let one: Row = block_on(select().fetch_one(&Mock::rows(users()))).unwrap();
    assert_eq!(one, row(1, "ann"));
}

#[test]
fn decode_error_is_propagated() {
    let bad = Row::new(
        ["id"].map(Box::<str>::from).into(),
        vec![1_i64.into_value()],
    );
    let db = Mock::rows(vec![row(1, "ann"), bad]);
    assert!(block_on(select().fetch_all::<User>(&db)).is_err());
    let wrong_type = Row::new(
        ["id", "name"].map(Box::<str>::from).into(),
        vec!["x".into_value(), "y".into_value()],
    );
    assert!(block_on(select().fetch_one::<User>(&Mock::rows(vec![wrong_type]))).is_err());
}

#[test]
fn executor_error_is_propagated_unchanged() {
    let db = Mock::failing();
    assert!(matches!(
        block_on(select().fetch_all::<Row>(&db)),
        Err(Error::Driver(_))
    ));
    assert!(matches!(
        block_on(select().fetch_one::<Row>(&db)),
        Err(Error::Driver(_))
    ));
    assert!(matches!(
        block_on(select().fetch_one_optional::<Row>(&db)),
        Err(Error::Driver(_))
    ));
    assert!(matches!(
        block_on(insert().execute(&db)),
        Err(Error::Driver(_))
    ));
}

// Insert

#[test]
fn insert_execute_sends_same_query_and_returns_affected() {
    let db = Mock::affecting(1);
    assert_eq!(block_on(insert().execute(&db)).unwrap(), 1);
    assert_eq!(
        db.calls(),
        vec![(Call::Execute, insert().into_query().unwrap())]
    );
}

#[test]
fn insert_returning_keeps_returning_in_query() {
    let db = Mock::rows(users());
    let q = || insert().returning(Column::new("id"));
    let got: Vec<User> = block_on(q().fetch_all(&db)).unwrap();
    assert_eq!(got.len(), 2);
    let Query::Insert(sent) = &db.calls()[0].1 else {
        panic!("expected Insert")
    };
    assert_eq!(sent.returning, vec![Expr::from(Column::new("id"))]);
    assert_eq!(
        db.calls(),
        vec![(Call::FetchAll, q().into_query().unwrap())]
    );
}

#[test]
fn insert_returning_can_still_execute() {
    let db = Mock::affecting(3);
    assert_eq!(
        block_on(insert().returning(Column::new("id")).execute(&db)).unwrap(),
        3
    );
}

#[test]
fn insert_returning_then_more_rows_and_returning() {
    let q = insert()
        .returning(Column::new("id"))
        .row(InsertRow::new().set(Column::new("name"), "bob".into_value()))
        .returning(Column::new("name"));
    let Query::Insert(i) = q.clone().into_query().unwrap() else {
        panic!()
    };
    assert_eq!(i.rows.len(), 2);
    assert_eq!(i.returning.len(), 2);
    let db = Mock::rows(users());
    let _: Vec<Row> = block_on(q.fetch_all(&db)).unwrap();
}

#[test]
fn invalid_insert_never_reaches_executor() {
    let db = Mock::affecting(1);
    assert_invalid(block_on(Insert::into("users").execute(&db)));
    assert_invalid(block_on(
        Insert::into("users")
            .returning(Column::new("id"))
            .fetch_all::<Row>(&db),
    ));
    assert!(db.calls().is_empty());
}

// Update

fn update() -> Update<flux_db_core::query::state::Filtered> {
    Update::table("users")
        .set(Column::new("name"), "z".into_value())
        .filter(id_is(1))
}

#[test]
fn update_execute_sends_same_query() {
    let db = Mock::affecting(1);
    assert_eq!(block_on(update().execute(&db)).unwrap(), 1);
    assert_eq!(
        db.calls(),
        vec![(Call::Execute, update().into_query().unwrap())]
    );
}

#[test]
fn update_all_execute() {
    let db = Mock::affecting(9);
    let q = || {
        Update::table("users")
            .set(Column::new("name"), "z".into_value())
            .all()
    };
    assert_eq!(block_on(q().execute(&db)).unwrap(), 9);
    assert_eq!(db.calls(), vec![(Call::Execute, q().into_query().unwrap())]);
}

#[test]
fn update_returning_fetches_and_keeps_filter() {
    let db = Mock::rows(users());
    let q = || update().returning(Column::new("id")).filter(id_is(2));
    let got: Option<User> = block_on(q().fetch_one_optional(&db)).unwrap();
    assert_eq!(got.unwrap().id, 1);
    let Query::Update(sent) = &db.calls()[0].1 else {
        panic!()
    };
    assert_eq!(sent.returning.len(), 1);
    assert_eq!(
        db.calls(),
        vec![(Call::FetchOneOptional, q().into_query().unwrap())]
    );
}

#[test]
fn update_without_set_never_reaches_executor() {
    let db = Mock::affecting(1);
    assert_invalid(block_on(
        Update::table("users").filter(id_is(1)).execute(&db),
    ));
    assert_invalid(block_on(
        Update::table("users")
            .all()
            .returning(Column::new("id"))
            .fetch_all::<Row>(&db),
    ));
    assert!(db.calls().is_empty());
}

// Delete

#[test]
fn delete_execute_sends_same_query() {
    let db = Mock::affecting(1);
    let q = || Delete::from("users").filter(id_is(1));
    assert_eq!(block_on(q().execute(&db)).unwrap(), 1);
    assert_eq!(db.calls(), vec![(Call::Execute, q().into_query().unwrap())]);
}

#[test]
fn delete_all_execute() {
    let db = Mock::affecting(5);
    assert_eq!(
        block_on(Delete::from("users").all().execute(&db)).unwrap(),
        5
    );
    assert_eq!(
        db.calls(),
        vec![(
            Call::Execute,
            Delete::from("users").all().into_query().unwrap()
        )]
    );
}

#[test]
fn delete_returning_fetch_one() {
    let db = Mock::rows(users());
    let q = || {
        Delete::from("users")
            .filter(id_is(1))
            .returning(Column::new("id"))
            .returning(Column::new("name"))
    };
    let got: User = block_on(q().fetch_one(&db)).unwrap();
    assert_eq!(got.name, "ann");
    let Query::Delete(sent) = &db.calls()[0].1 else {
        panic!()
    };
    assert_eq!(sent.returning.len(), 2);
}

#[test]
fn returning_marker_does_not_change_the_query() {
    // Same data with and without the typestate hop: `R` must be erased in `Query`.
    let plain = Delete::from("users").filter(id_is(1)).into_query().unwrap();
    let Query::Delete(mut d) = Delete::from("users")
        .filter(id_is(1))
        .returning(Column::new("id"))
        .into_query()
        .unwrap()
    else {
        panic!()
    };
    d.returning.clear();
    assert_eq!(Query::Delete(d), plain);
}

// Raw

#[test]
fn raw_all_four_shortcuts() {
    let q = || RawQuery::new("SELECT id, name FROM users WHERE id = $1").bind(1_i64);
    let db = Mock::rows(users());
    assert_eq!(block_on(q().fetch_all::<User>(&db)).unwrap().len(), 2);
    assert_eq!(block_on(q().fetch_one::<User>(&db)).unwrap().id, 1);
    assert!(
        block_on(q().fetch_one_optional::<User>(&db))
            .unwrap()
            .is_some()
    );
    assert_eq!(block_on(q().execute(&db)).unwrap(), 0);
    let expected = q().into_query().unwrap();
    let calls = db.calls();
    assert_eq!(calls.len(), 4);
    assert!(calls.iter().all(|(_, sent)| *sent == expected));
    assert_eq!(
        calls.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        [
            Call::FetchAll,
            Call::FetchOneOptional,
            Call::FetchOneOptional,
            Call::Execute
        ]
    );
}

#[test]
fn raw_params_are_sent_untouched() {
    let db = Mock::default();
    block_on(
        RawQuery::new("x")
            .bind(None::<i64>)
            .bind("'; DROP TABLE users; --")
            .execute(&db),
    )
    .unwrap();
    let Query::Raw(r) = &db.calls()[0].1 else {
        panic!()
    };
    assert_eq!(
        r.params,
        vec![Value::Null, "'; DROP TABLE users; --".into_value()]
    );
}

// Futures are Send (usable on multi-threaded runtimes).

fn assert_send<T: Send>(_: T) {}

#[test]
fn shortcut_futures_are_send() {
    let db = Mock::default();
    assert_send(select().fetch_all::<User>(&db));
    assert_send(select().fetch_one::<Row>(&db));
    assert_send(insert().execute(&db));
    assert_send(
        update()
            .returning(Column::new("id"))
            .fetch_one_optional::<Row>(&db),
    );
    assert_send(Delete::from("t").all().execute(&db));
    assert_send(RawQuery::new("x").fetch_all::<Row>(&db));
}

// Stress

#[test]
fn stress_many_shortcuts_one_executor() {
    let db = Mock::rows(users());
    for i in 0..5_000_i64 {
        let q = Select::from("users").filter(id_is(i));
        let _: Vec<User> = block_on(q.fetch_all(&db)).unwrap();
    }
    let calls = db.calls();
    assert_eq!(calls.len(), 5_000);
    assert_eq!(
        calls[4_999].1,
        Select::from("users")
            .filter(id_is(4_999))
            .into_query()
            .unwrap()
    );
}

#[test]
fn stress_large_result_decode() {
    let rows: Vec<Row> = (0..20_000).map(|i| row(i, "n")).collect();
    let got: Vec<User> = block_on(select().fetch_all(&Mock::rows(rows))).unwrap();
    assert_eq!(got.len(), 20_000);
    assert_eq!(got[19_999].id, 19_999);
}
