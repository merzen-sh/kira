//! SQLite connection pool and schema ownership.

use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::StoreError;
use crate::audit::AuditRepository;
use crate::monitor::MonitorRepository;
use crate::reports::ReportRepository;
use crate::users::UserRepository;

/// Shared SQLite connection pool and schema owner.
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Open the SQLite database at `url`, applying pending migrations.
    ///
    /// # Errors
    ///
    /// Returns an error when the URL is invalid, the database cannot be opened,
    /// or a migration fails.
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            // The rollback journal fsyncs twice per write; WAL with NORMAL is
            // the standard SQLite pairing and keeps the database consistent
            // across power loss at the cost of possibly losing recent writes.
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal);

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

    /// Host resource sample persistence.
    #[must_use]
    pub fn monitor(&self) -> MonitorRepository {
        MonitorRepository::new(self.clone())
    }

    /// Guild membership persistence and sync bookkeeping.
    #[must_use]
    pub fn users(&self) -> UserRepository {
        UserRepository::new(self.clone())
    }

    /// Generic audit event persistence.
    #[must_use]
    pub fn audit(&self) -> AuditRepository {
        AuditRepository::new(self.clone())
    }

    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub(crate) async fn begin(&self) -> Result<Transaction<'_, Sqlite>, StoreError> {
        Ok(self.pool.begin().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn file_database_uses_wal_and_normal_synchronous() -> Result<(), StoreError> {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("test.db");
        let database = Database::connect(&format!("sqlite:{}", path.display())).await?;

        let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(database.pool())
            .await?;
        let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
            .fetch_one(database.pool())
            .await?;

        assert_eq!(mode, "wal");
        assert_eq!(synchronous, 1);
        Ok(())
    }
}
