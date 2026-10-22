//! Running queries.

use std::future::Future;

use futures_core::Stream;

use crate::query::Query;
use crate::types::error::{Error, Result};
use crate::types::row::Row;

/// Anything that can run a [`Query`]: a pool, a connection or a transaction.
pub trait Executor: Send + Sync {
    /// Runs a statement and returns the number of affected rows.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Driver`] when the database rejects the statement or
    /// the connection fails.
    fn execute(&self, query: &Query) -> impl Future<Output = Result<u64>> + Send;

    /// Streams rows as they arrive.
    ///
    /// Each item is a [`Result`], so a failure mid-stream ends the stream with
    /// an error instead of dropping rows silently.
    fn fetch<'a>(&'a self, query: &'a Query) -> impl Stream<Item = Result<Row>> + Send + 'a;

    /// Collects every row.
    ///
    /// # Errors
    ///
    /// Returns the first error the stream yields, typically [`Error::Driver`].
    fn fetch_all(&self, query: &Query) -> impl Future<Output = Result<Vec<Row>>> + Send;

    /// The first row, if any; `Ok(None)` when the query returns no rows.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Driver`] when the query fails.
    fn fetch_one_optional(&self, query: &Query)
    -> impl Future<Output = Result<Option<Row>>> + Send;

    /// The first row, failing when there is none.
    ///
    /// Provided in terms of [`fetch_one_optional`](Self::fetch_one_optional).
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] when the query returns no rows, or any
    /// error from [`fetch_one_optional`](Self::fetch_one_optional).
    fn fetch_one(&self, query: &Query) -> impl Future<Output = Result<Row>> + Send {
        async move { self.fetch_one_optional(query).await?.ok_or(Error::NotFound) }
    }
}
