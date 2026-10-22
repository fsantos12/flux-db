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
    fn acquire(&self) -> impl Future<Output = Result<Self::Connection>> + Send;
}