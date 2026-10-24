//! Core of flux-db: shared [`types`], the query [`ast`], query builders in
//! [`query`] and the [`driver`] traits.
//!
//! This crate is database-agnostic; driver implementations and SQL rendering
//! live in other crates.

pub mod ast;
pub mod driver;
pub mod query;
pub mod types;

// Flat re-exports of the most used items, so callers can write
// `flux_db_core::Select` instead of the full module path.
pub use ast::expr::{BinaryOp, Expr, Func, LikePattern};
pub use ast::group::GroupBy;
pub use ast::order::{Direction, NullsOrder, OrderBy};
pub use driver::{Connection, Driver, Executor, Transaction};
pub use query::delete::Delete;
pub use query::insert::{Insert, InsertRow};
pub use query::raw::RawQuery;
pub use query::select::Select;
pub use query::state::{All, Executable, Filtered, NoReturning, Returning, Unfiltered};
pub use query::update::{Assignment, Update};
pub use query::{DeleteQuery, IntoQuery, Query, UpdateQuery};
pub use types::column::{Column, TypedColumn};
pub use types::error::{Error, Result};
pub use types::row::{FromRow, Row};
pub use types::value::{FromValue, IntoValue, Value};
