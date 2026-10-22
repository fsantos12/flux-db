//! A dedicated connection.

use std::future::Future;

use crate::driver::{Executor, Transaction};
use crate::types::error::Result;

/// One connection; the only place a transaction can start.
pub trait Connection: Executor {
    /// A transaction borrowing this connection.
    type Transaction<'c>: Transaction
    where
        Self: 'c;

    /// Starts a transaction (`BEGIN`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Driver`](crate::types::error::Error::Driver) when the
    /// database refuses to start the transaction.
    fn begin(&mut self) -> impl Future<Output = Result<Self::Transaction<'_>>> + Send;
}
