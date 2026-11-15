//! Errors produced by SQLite storage.

use thiserror::Error;

/// Errors produced while accessing SQLite storage.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StoreError {
    /// The database could not be opened or queried.
    #[error("database error")]
    Database(#[from] sqlx::Error),

    /// The database schema migrations failed.
    #[error("database migration failed")]
    Migration(#[from] sqlx::migrate::MigrateError),
}
