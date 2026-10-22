//! Query builders (`select`, `insert`, `update`, `delete`, `raw`), built on
//! the [`ast`](crate::ast) nodes.
//!
//! Builders only describe a statement; rendering and execution live in other
//! crates. Every builder turns into a [`Query`] through [`IntoQuery`], which
//! validates it first.

pub mod delete;
pub mod insert;
pub mod raw;
pub mod select;
pub mod state;
pub mod update;

use crate::ast::expr::Expr;
use crate::types::column::Column;
use crate::types::error::Result;

use self::delete::Delete;
use self::insert::Insert;
use self::raw::RawQuery;
use self::select::Select;
use self::state::Executable;
use self::update::{Assignment, Update};

/// A complete, validated statement, ready to render.
///
/// Only built through [`IntoQuery`], so unfiltered writes and invalid
/// inserts or updates can never reach a renderer.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Query {
    /// `SELECT ...`
    Select(Select),
    /// `INSERT INTO ...`
    Insert(Insert),
    /// `UPDATE ...`
    Update(UpdateQuery),
    /// `DELETE FROM ...`
    Delete(DeleteQuery),
    /// Raw SQL with bound params.
    Raw(RawQuery),
}

/// An `UPDATE` with its typestate stripped; only reachable from an executable state.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateQuery {
    /// Target table.
    pub table: &'static str,
    /// `SET` assignments, in call order.
    pub assignments: Vec<Assignment>,
    /// `WHERE` clause; `None` means every row (`.all()`).
    pub filter: Option<Expr>,
    /// `RETURNING` columns or expressions.
    pub returning: Vec<Expr>,
}

/// A `DELETE` with its typestate stripped; only reachable from an executable state.
#[derive(Debug, Clone, PartialEq)]
pub struct DeleteQuery {
    /// Target table.
    pub table: &'static str,
    /// `WHERE` clause; `None` means every row (`.all()`).
    pub filter: Option<Expr>,
    /// `RETURNING` columns or expressions.
    pub returning: Vec<Expr>,
}

/// Turns a builder into a [`Query`], validating it first.
pub trait IntoQuery {
    /// Validates the builder and converts it into a [`Query`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidQuery`](crate::types::error::Error::InvalidQuery)
    /// when the builder fails its own `validate()`.
    fn into_query(self) -> Result<Query>;
}

impl IntoQuery for Query {
    fn into_query(self) -> Result<Query> {
        Ok(self)
    }
}

impl IntoQuery for Select {
    fn into_query(self) -> Result<Query> {
        Ok(Query::Select(self))
    }
}

impl IntoQuery for RawQuery {
    fn into_query(self) -> Result<Query> {
        Ok(Query::Raw(self))
    }
}

impl IntoQuery for Insert {
    fn into_query(self) -> Result<Query> {
        self.validate()?;
        Ok(Query::Insert(self))
    }
}

impl<S: Executable> IntoQuery for Update<S> {
    fn into_query(self) -> Result<Query> {
        self.validate()?;
        Ok(Query::Update(UpdateQuery {
            table: self.table,
            assignments: self.assignments,
            filter: self.filter,
            returning: self.returning,
        }))
    }
}

impl<S: Executable> IntoQuery for Delete<S> {
    // No `validate()` here: a `DELETE` has nothing to check beyond its
    // typestate, and the `Executable` bound already rules out `Unfiltered`.
    fn into_query(self) -> Result<Query> {
        Ok(Query::Delete(DeleteQuery {
            table: self.table,
            filter: self.filter,
            returning: self.returning,
        }))
    }
}

/// Returns the first column name that appears more than once.
///
/// Compares by name only, so `users.name` and `name` count as the same
/// column. A linear scan beats hashing at real row widths.
pub(crate) fn duplicate_column<'a>(
    columns: impl IntoIterator<Item = &'a Column>,
) -> Option<&'static str> {
    // Intentionally a linear Vec scan: column counts are small, so this beats
    // a HashSet in practice despite being O(n^2).
    let mut seen: Vec<&'static str> = Vec::new();
    columns.into_iter().map(Column::name).find(|name| {
        let duplicate = seen.contains(name);
        seen.push(name);
        duplicate
    })
}
