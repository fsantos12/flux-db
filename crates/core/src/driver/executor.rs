//! Running queries.

use std::future::Future;

use futures_core::Stream;

use crate::query::Query;
use crate::types::error::{Error, Result};
use crate::types::row::Row;

/// Anything that can run a [`Query`]: a pool, a connection or a transaction.
pub trait Executor: Send + Sync {
    /// Runs a statement and returns the number of affected rows.
    fn execute(&self, query: &Query) -> impl Future<Output = Result<u64>> + Send;

    /// Streams rows as they arrive.
    fn fetch<'a>(&'a self, query: &'a Query) -> impl Stream<Item = Result<Row>> + Send + 'a;

    /// Collects every row.
    fn fetch_all(&self, query: &Query) -> impl Future<Output = Result<Vec<Row>>> + Send;

    /// The first row, if any.
    fn fetch_one_optional(&self, query: &Query) -> impl Future<Output = Result<Option<Row>>> + Send;

    /// Exactly the first row; [`Error::NotFound`] when there is none.
    fn fetch_one(&self, query: &Query) -> impl Future<Output = Result<Row>> + Send {
        async move { self.fetch_one_optional(query).await?.ok_or(Error::NotFound) }
    }
}