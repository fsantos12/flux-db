//! Driver traits only; implementations live in driver crates (e.g. `flux-db-postgres`).
//!
//! Runtime-agnostic: no tokio types here. Pools, connections and transactions
//! all implement [`Executor`], so Active Record methods take `&impl Executor`.

pub mod connection;
// `driver::driver` mirrors the trait name; renaming the file would change the
// public path `flux_db_core::driver::driver`, so the lint is allowed instead.
#[allow(clippy::module_inception)]
pub mod driver;
pub mod executor;
pub mod transaction;

pub use connection::Connection;
pub use driver::Driver;
pub use executor::Executor;
pub use transaction::Transaction;
