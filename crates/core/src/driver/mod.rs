//! Driver traits only; implementations live in driver crates (e.g. `flux-db-postgres`).
//!
//! Runtime-agnostic: no tokio types here. Pools, connections and transactions
//! all implement [`Executor`], so Active Record methods take `&impl Executor`.

pub mod connection;
pub mod driver;
pub mod executor;
pub mod transaction;

pub use connection::Connection;
pub use driver::Driver;
pub use executor::Executor;
pub use transaction::Transaction;