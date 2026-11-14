//! SQLite infrastructure and repositories shared by the bot and server.

pub mod reports;
pub mod users;

use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Sqlite, SqlitePool, Transaction};
use thiserror::Error;

use reports::ReportRepository;
use users::UserRepository;

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

/// Shared SQLite connection pool and schema owner.
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Open the SQLite database at `url`, applying pending migrations.
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);

        // SQLite has a single writer, and an in-memory database is per
        // connection. One pooled connection keeps both cases predictable;
        // never recycle it, or an in-memory database loses its schema.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .min_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await?;
        // The migrations live at the workspace root; this crate is one level
        // deeper than the workspace crates directory.
        sqlx::migrate!("../../migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    /// Bug report persistence.
    #[must_use]
    pub fn reports(&self) -> ReportRepository {
        ReportRepository::new(self.clone())
    }

    /// Guild membership persistence and sync bookkeeping.
    #[must_use]
    pub fn users(&self) -> UserRepository {
        UserRepository::new(self.clone())
    }

    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub(crate) async fn begin(&self) -> Result<Transaction<'_, Sqlite>, StoreError> {
        Ok(self.pool.begin().await?)
    }
}

/// Escape LIKE wildcards so search text matches literally.
pub(crate) fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
pub(crate) async fn memory() -> Database {
    Database::connect("sqlite::memory:")
        .await
        .expect("test database")
}
