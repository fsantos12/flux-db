//! Transactions.

use std::future::Future;

use crate::driver::Executor;
use crate::types::error::Result;

/// An open transaction. Must be committed explicitly; dropping it rolls back.
///
/// No `begin` here: savepoints and nested transactions are out of scope for v1.
pub trait Transaction: Executor {
    /// `COMMIT`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Driver`](crate::types::error::Error::Driver) when
    /// the commit fails; the transaction is then rolled back by the database.
    fn commit(self) -> impl Future<Output = Result<()>> + Send;

    /// `ROLLBACK`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Driver`](crate::types::error::Error::Driver) when
    /// the rollback fails.
    fn rollback(self) -> impl Future<Output = Result<()>> + Send;
}
