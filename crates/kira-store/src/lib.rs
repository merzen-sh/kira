//! SQLite infrastructure and repositories shared by the bot and server.

mod database;
mod errors;
pub mod monitor;
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
