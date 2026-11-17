//! SQLite infrastructure and repositories shared by the bot and server.
//!
//! Repositories return [`StoreError`] when a query or migration fails.

pub mod audit;
mod database;
mod errors;
pub mod monitor;
pub mod presence;
mod query;
pub mod reports;
pub mod users;

pub use database::Database;
pub use errors::StoreError;

#[cfg(test)]
pub(crate) async fn memory() -> Database {
    Database::connect("sqlite::memory:")
        .await
        .expect("test database")
}
