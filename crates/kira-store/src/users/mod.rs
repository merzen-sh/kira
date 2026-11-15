//! Guild membership persistence and metadata sync bookkeeping.

mod sync;

use kira_protocol::{GuildUser, GuildUserList, GuildUserStats};
use sqlx::FromRow;

use crate::query::{escape_like, page_offset};
use crate::{Database, StoreError};

const USER_COLUMNS: &str = "user_id, user_name, display_name, status, join_at, leave_at";

const FILTER: &str = "(? = '' OR user_id LIKE ? ESCAPE '\\' OR user_name LIKE ? ESCAPE '\\' OR display_name LIKE ? ESCAPE '\\') AND (? IS NULL OR status = ?)";

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ','now')";

const NEXT_SYNC: &str = "strftime('%Y-%m-%dT%H:%M:%fZ','now','+3 days')";

/// A running sync older than this is considered abandoned.
const STALE_LOCK: &str = "strftime('%Y-%m-%dT%H:%M:%fZ','now','-30 minutes')";

/// Queries for guild members, join/leave history, and sync state.
#[derive(Clone)]
pub struct UserRepository {
    db: Database,
}

impl UserRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    /// Record a join or rejoin, reactivating the user in one transaction.
    pub async fn join(
        &self,
        user_id: &str,
        user_name: &str,
        display_name: &str,
    ) -> Result<(), StoreError> {
        let mut tx = self.db.begin().await?;

        let sql = format!(
            "INSERT INTO guild_users \
                (user_id, user_name, display_name, status, join_at, leave_at) \
             VALUES (?, ?, ?, 'active', {NOW}, NULL) \
             ON CONFLICT(user_id) DO UPDATE SET \
                user_name = excluded.user_name, \
                display_name = excluded.display_name, \
                status = 'active', \
                join_at = excluded.join_at, \
                leave_at = NULL"
        );
        sqlx::query(&sql)
            .bind(user_id)
            .bind(user_name)
            .bind(display_name)
            .execute(&mut *tx)
            .await?;

        insert_event(&mut tx, user_id, "join").await?;
        tx.commit().await?;

        Ok(())
    }

    /// Mark a user as having left, preserving the existing record.
    pub async fn leave(
        &self,
        user_id: &str,
        user_name: &str,
        display_name: &str,
    ) -> Result<(), StoreError> {
        let mut tx = self.db.begin().await?;

        let sql = format!(
            "INSERT INTO guild_users \
                (user_id, user_name, display_name, status, join_at, leave_at) \
             VALUES (?, ?, ?, 'left', {NOW}, {NOW}) \
             ON CONFLICT(user_id) DO UPDATE SET \
                user_name = excluded.user_name, \
                display_name = excluded.display_name, \
                status = 'left', \
                leave_at = excluded.leave_at"
        );
        sqlx::query(&sql)
            .bind(user_id)
            .bind(user_name)
            .bind(display_name)
            .execute(&mut *tx)
            .await?;

        insert_event(&mut tx, user_id, "leave").await?;
        tx.commit().await?;

        Ok(())
    }

    /// Record a member found during the startup reconciliation.
    ///
    /// Existing active users keep their original `join_at` and get no duplicate
    /// Join event. Users who rejoined while the bot was offline are reactivated
    /// and get a Join event. `snapshot` guards against a leave that was handled
    /// after the reconciliation snapshot was taken.
    pub async fn import_member(
        &self,
        user_id: &str,
        user_name: &str,
        display_name: &str,
        joined_at: Option<&str>,
        snapshot: &str,
    ) -> Result<(), StoreError> {
        let mut tx = self.db.begin().await?;

        let existing: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT status, leave_at FROM guild_users WHERE user_id = ?")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

        match existing {
            None => {
                let sql = format!(
                    "INSERT INTO guild_users \
                        (user_id, user_name, display_name, status, join_at, leave_at) \
                     VALUES (?, ?, ?, 'active', COALESCE(?, {NOW}), NULL)"
                );
                sqlx::query(&sql)
                    .bind(user_id)
                    .bind(user_name)
                    .bind(display_name)
                    .bind(joined_at)
                    .execute(&mut *tx)
                    .await?;
                insert_event(&mut tx, user_id, "join").await?;
            }
            Some((status, leave_at)) if status == "left" => {
                if leave_at
                    .as_deref()
                    .is_some_and(|left_at| left_at >= snapshot)
                {
                    // A leave arrived after the snapshot; the member page is stale.
                    tx.commit().await?;
                    return Ok(());
                }

                let sql = format!(
                    "UPDATE guild_users \
                     SET user_name = ?, display_name = ?, status = 'active', \
                         join_at = COALESCE(?, {NOW}), leave_at = NULL \
                     WHERE user_id = ?"
                );
                sqlx::query(&sql)
                    .bind(user_name)
                    .bind(display_name)
                    .bind(joined_at)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
                insert_event(&mut tx, user_id, "join").await?;
            }
            Some(_) => {
                sqlx::query(
                    "UPDATE guild_users SET user_name = ?, display_name = ? \
                     WHERE user_id = ? AND status = 'active'",
                )
                .bind(user_name)
                .bind(display_name)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        Ok(())
    }

    /// Capture the reconciliation snapshot time.
    pub async fn snapshot_time(&self) -> Result<String, StoreError> {
        Ok(sqlx::query_scalar(&format!("SELECT {NOW}"))
            .fetch_one(self.db.pool())
            .await?)
    }

    /// IDs of every user currently tracked as active.
    pub async fn active_ids(&self) -> Result<Vec<String>, StoreError> {
        Ok(
            sqlx::query_scalar("SELECT user_id FROM guild_users WHERE status = 'active'")
                .fetch_all(self.db.pool())
                .await?,
        )
    }

    /// Mark tracked active users that are missing from the guild as left.
    ///
    /// Users who joined after `snapshot` are skipped because their Join event
    /// may have raced with the member pages.
    pub async fn mark_left(&self, user_ids: &[String], snapshot: &str) -> Result<(), StoreError> {
        if user_ids.is_empty() {
            return Ok(());
        }

        let mut tx = self.db.begin().await?;
        let sql = format!(
            "UPDATE guild_users SET status = 'left', leave_at = {NOW} \
             WHERE user_id = ? AND status = 'active' AND join_at < ?"
        );

        for user_id in user_ids {
            let result = sqlx::query(&sql)
                .bind(user_id)
                .bind(snapshot)
                .execute(&mut *tx)
                .await?;

            if result.rows_affected() > 0 {
                insert_event(&mut tx, user_id, "leave").await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// Count every tracked user and how many are active.
    pub async fn stats(&self) -> Result<GuildUserStats, StoreError> {
        let (total, active): (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*), \
                COALESCE(SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END), 0) \
             FROM guild_users",
        )
        .fetch_one(self.db.pool())
        .await?;

        Ok(GuildUserStats::new(
            u64::try_from(total).unwrap_or_default(),
            u64::try_from(active).unwrap_or_default(),
        ))
    }

    /// List users newest first, optionally filtered by search text and status.
    pub async fn list(
        &self,
        page: u32,
        page_limit: u32,
        search: Option<&str>,
        status: Option<&str>,
    ) -> Result<GuildUserList, StoreError> {
        let search = search.map(str::trim).filter(|value| !value.is_empty());
        let search = search.unwrap_or_default();
        let pattern = format!("%{}%", escape_like(search));
        let (limit, offset) = page_offset(page, page_limit);

        let count_sql = format!("SELECT COUNT(*) FROM guild_users WHERE {FILTER}");
        let total: i64 = sqlx::query_scalar(&count_sql)
            .bind(search)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(status)
            .bind(status)
            .fetch_one(self.db.pool())
            .await?;

        let list_sql = format!(
            "SELECT {USER_COLUMNS} FROM guild_users WHERE {FILTER} \
             ORDER BY join_at DESC, user_id ASC LIMIT ? OFFSET ?"
        );
        let rows = sqlx::query_as::<_, GuildUserRow>(&list_sql)
            .bind(search)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(status)
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(self.db.pool())
            .await?;

        Ok(GuildUserList::new(
            rows.into_iter().map(GuildUser::from).collect(),
            u64::try_from(total).unwrap_or_default(),
            page,
            page_limit,
        ))
    }
}

async fn insert_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    user_id: &str,
    event_type: &str,
) -> Result<(), StoreError> {
    let sql = format!(
        "INSERT INTO guild_user_events (user_id, event_type, created_at) \
         VALUES (?, ?, {NOW})"
    );

    sqlx::query(&sql)
        .bind(user_id)
        .bind(event_type)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

#[derive(Debug, FromRow)]
struct GuildUserRow {
    user_id: String,
    user_name: String,
    display_name: String,
    status: String,
    join_at: String,
    leave_at: Option<String>,
}

impl From<GuildUserRow> for GuildUser {
    fn from(row: GuildUserRow) -> Self {
        Self::new(
            row.user_id,
            row.user_name,
            row.display_name,
            row.status,
            row.join_at,
            row.leave_at,
        )
    }
}

#[cfg(test)]
mod tests;
