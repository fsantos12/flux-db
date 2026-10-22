//! Transactions.

use std::future::Future;

use crate::driver::Executor;
use crate::types::error::Result;

/// An open transaction. Must be committed explicitly; dropping it rolls back.
///
/// No `begin` here: savepoints and nested transactions are out of scope for v1.
pub trait Transaction: Executor {
    /// `COMMIT`.
    fn commit(self) -> impl Future<Output = Result<()>> + Send;

    /// `ROLLBACK`.
    fn rollback(self) -> impl Future<Output = Result<()>> + Send;
}