//! Entry point of a database backend.

use std::future::Future;

use crate::driver::{Connection, Executor};
use crate::types::error::Result;

/// A database backend, usually a pool. It can run queries itself and hand
/// out dedicated connections.
pub trait Driver: Executor + 'static {
    /// A connection checked out from this driver.
    type Connection: Connection;

    /// Checks out a connection (waits on the pool if needed).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Driver`](crate::types::error::Error::Driver) when no
    /// connection can be opened or the pool times out.
    fn acquire(&self) -> impl Future<Output = Result<Self::Connection>> + Send;
}
