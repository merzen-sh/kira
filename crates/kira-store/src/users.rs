//! Guild membership persistence and metadata sync bookkeeping.

use kira_protocol::{GuildUser, GuildUserList};
use sqlx::FromRow;

use crate::{Database, StoreError, escape_like};

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
    /// and get a Join event.
    pub async fn import_member(
        &self,
        user_id: &str,
        user_name: &str,
        display_name: &str,
    ) -> Result<(), StoreError> {
        let mut tx = self.db.begin().await?;

        let was_active: Option<bool> =
            sqlx::query_scalar("SELECT status = 'active' FROM guild_users WHERE user_id = ?")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

        let sql = format!(
            "INSERT INTO guild_users \
                (user_id, user_name, display_name, status, join_at, leave_at) \
             VALUES (?, ?, ?, 'active', {NOW}, NULL) \
             ON CONFLICT(user_id) DO UPDATE SET \
                join_at = CASE WHEN guild_users.status = 'left' \
                    THEN excluded.join_at ELSE guild_users.join_at END, \
                leave_at = CASE WHEN guild_users.status = 'left' \
                    THEN NULL ELSE guild_users.leave_at END, \
                status = 'active', \
                user_name = excluded.user_name, \
                display_name = excluded.display_name"
        );
        sqlx::query(&sql)
            .bind(user_id)
            .bind(user_name)
            .bind(display_name)
            .execute(&mut *tx)
            .await?;

        if !was_active.unwrap_or(false) {
            insert_event(&mut tx, user_id, "join").await?;
        }

        tx.commit().await?;

        Ok(())
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
    pub async fn mark_left(&self, user_ids: &[String]) -> Result<(), StoreError> {
        if user_ids.is_empty() {
            return Ok(());
        }

        let mut tx = self.db.begin().await?;
        let sql = format!(
            "UPDATE guild_users SET status = 'left', leave_at = {NOW} \
             WHERE user_id = ? AND status = 'active'"
        );

        for user_id in user_ids {
            let result = sqlx::query(&sql).bind(user_id).execute(&mut *tx).await?;

            if result.rows_affected() > 0 {
                insert_event(&mut tx, user_id, "leave").await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// Push a user's next sync out without recording a successful refresh.
    pub async fn defer_sync(&self, user_id: &str) -> Result<(), StoreError> {
        let sql = format!(
            "UPDATE guild_users SET next_sync_at = {NEXT_SYNC} \
             WHERE user_id = ? AND status = 'active'"
        );

        sqlx::query(&sql)
            .bind(user_id)
            .execute(self.db.pool())
            .await?;

        Ok(())
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
        let limit = i64::from(page_limit);
        let offset = i64::from(page.saturating_sub(1)) * limit;

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

    /// Active users due for a metadata refresh, ordered by ID.
    ///
    /// `after` is the checkpoint from the previous batch.
    pub async fn due_for_sync(&self, after: &str, limit: u32) -> Result<Vec<String>, StoreError> {
        let sql = format!(
            "SELECT user_id FROM guild_users \
             WHERE status = 'active' \
               AND user_id > ? \
               AND (next_sync_at IS NULL OR next_sync_at <= {NOW}) \
             ORDER BY user_id LIMIT ?"
        );

        Ok(sqlx::query_scalar(&sql)
            .bind(after)
            .bind(i64::from(limit))
            .fetch_all(self.db.pool())
            .await?)
    }

    /// Apply refreshed metadata after a successful Discord fetch.
    ///
    /// Returns `false` when the user is no longer active, so a concurrent
    /// leave is never overwritten.
    pub async fn record_sync(
        &self,
        user_id: &str,
        user_name: &str,
        display_name: &str,
    ) -> Result<bool, StoreError> {
        let sql = format!(
            "UPDATE guild_users \
             SET user_name = ?, display_name = ?, last_synced_at = {NOW}, \
                 next_sync_at = {NEXT_SYNC} \
             WHERE user_id = ? AND status = 'active'"
        );

        let result = sqlx::query(&sql)
            .bind(user_name)
            .bind(display_name)
            .bind(user_id)
            .execute(self.db.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Claim the sync lock and return its checkpoint, or `None` when another
    /// sync is already running.
    pub async fn claim_sync(&self) -> Result<Option<String>, StoreError> {
        let sql = format!(
            "UPDATE guild_sync_state \
             SET running = 1, locked_at = {NOW}, updated_at = {NOW} \
             WHERE id = 1 \
               AND (running = 0 OR locked_at IS NULL OR locked_at <= {STALE_LOCK})"
        );

        let result = sqlx::query(&sql).execute(self.db.pool()).await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        let cursor: String = sqlx::query_scalar("SELECT cursor FROM guild_sync_state WHERE id = 1")
            .fetch_one(self.db.pool())
            .await?;

        Ok(Some(cursor))
    }

    /// Save the checkpoint for the next run.
    pub async fn save_sync_cursor(&self, cursor: &str) -> Result<(), StoreError> {
        let sql = format!(
            "UPDATE guild_sync_state \
             SET cursor = ?, locked_at = {NOW}, updated_at = {NOW} WHERE id = 1"
        );

        sqlx::query(&sql)
            .bind(cursor)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Release the lock but keep the checkpoint so the next run resumes.
    pub async fn release_sync(&self) -> Result<(), StoreError> {
        let sql = format!(
            "UPDATE guild_sync_state \
             SET running = 0, locked_at = NULL, updated_at = {NOW} \
             WHERE id = 1"
        );

        sqlx::query(&sql).execute(self.db.pool()).await?;
        Ok(())
    }

    /// Release the lock and clear the checkpoint after a complete pass.
    pub async fn finish_sync(&self) -> Result<(), StoreError> {
        let sql = format!(
            "UPDATE guild_sync_state \
             SET running = 0, cursor = '', locked_at = NULL, updated_at = {NOW} \
             WHERE id = 1"
        );

        sqlx::query(&sql).execute(self.db.pool()).await?;
        Ok(())
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
mod tests {
    use crate::StoreError;

    use super::UserRepository;

    async fn repo() -> UserRepository {
        crate::memory().await.users()
    }

    async fn event_count(repo: &UserRepository, user_id: &str) -> Result<i64, StoreError> {
        let sql = "SELECT COUNT(*) FROM guild_user_events WHERE user_id = ?";
        Ok(sqlx::query_scalar(sql)
            .bind(user_id)
            .fetch_one(repo.db.pool())
            .await?)
    }

    #[tokio::test]
    async fn join_activates_a_user_and_records_an_event() -> Result<(), StoreError> {
        let users = repo().await;

        users.join("1", "alice", "Alice").await?;

        let page = users.list(1, 20, None, None).await?;
        assert_eq!(page.total, 1);
        assert_eq!(page.users[0].status, "active");
        assert_eq!(page.users[0].user_name, "alice");
        assert_eq!(page.users[0].display_name, "Alice");
        assert!(page.users[0].leave_at.is_none());
        assert!(!page.users[0].join_at.is_empty());
        assert_eq!(event_count(&users, "1").await?, 1);
        Ok(())
    }

    #[tokio::test]
    async fn leave_preserves_data_and_rejoin_reactivates() -> Result<(), StoreError> {
        let users = repo().await;

        users.join("1", "alice", "Alice").await?;
        users.leave("1", "alice", "Alice").await?;

        let left = users.list(1, 20, None, Some("left")).await?;
        assert_eq!(left.total, 1);
        assert!(left.users[0].leave_at.is_some());

        users.join("1", "alice2", "Alice Two").await?;

        let active = users.list(1, 20, None, Some("active")).await?;
        assert_eq!(active.total, 1);
        assert_eq!(active.users[0].user_name, "alice2");
        assert_eq!(active.users[0].display_name, "Alice Two");
        assert!(active.users[0].leave_at.is_none());
        assert_eq!(event_count(&users, "1").await?, 3);
        Ok(())
    }

    #[tokio::test]
    async fn list_searches_filters_and_paginates() -> Result<(), StoreError> {
        let users = repo().await;
        users.join("1", "alice", "Alice").await?;
        users.join("2", "bob", "Bob").await?;
        users.leave("2", "bob", "Bob").await?;

        let all = users.list(1, 20, None, None).await?;
        assert_eq!(all.total, 2);

        let searched = users.list(1, 20, Some("bob"), None).await?;
        assert_eq!(searched.total, 1);
        assert_eq!(searched.users[0].user_id, "2");

        let active = users.list(1, 20, None, Some("active")).await?;
        assert_eq!(active.total, 1);
        assert_eq!(active.users[0].user_id, "1");

        let percent = users.list(1, 20, Some("%"), None).await?;
        assert_eq!(percent.total, 0);

        let second = users.list(2, 1, None, None).await?;
        assert_eq!(second.total, 2);
        assert_eq!(second.users.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn reconciliation_imports_reactivates_and_marks_missing_left() -> Result<(), StoreError> {
        let users = repo().await;
        users.join("1", "alice", "Alice").await?;
        users.join("2", "bob", "Bob").await?;
        users.leave("2", "bob", "Bob").await?;

        users.import_member("1", "alice", "Alice").await?;
        users.import_member("2", "bob", "Bob").await?;

        // Re-importing an active user does not duplicate Join history.
        assert_eq!(event_count(&users, "1").await?, 1);
        // Reactivating a left user appends a Join event.
        assert_eq!(event_count(&users, "2").await?, 3);

        users.mark_left(&["1".to_owned()]).await?;

        let left = users.list(1, 20, None, Some("left")).await?;
        assert_eq!(left.total, 1);
        assert_eq!(left.users[0].user_id, "1");
        assert_eq!(event_count(&users, "1").await?, 2);
        assert_eq!(users.active_ids().await?, vec!["2"]);
        Ok(())
    }

    #[tokio::test]
    async fn sync_only_selects_active_users_due_after_the_cursor() -> Result<(), StoreError> {
        let users = repo().await;
        users.join("1", "alice", "Alice").await?;
        users.join("2", "bob", "Bob").await?;
        users.leave("2", "bob", "Bob").await?;

        let due = users.due_for_sync("", 10).await?;
        assert_eq!(due, vec!["1"]);

        assert!(users.due_for_sync("1", 10).await?.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn successful_sync_sets_the_cooldown_and_respects_a_leave() -> Result<(), StoreError> {
        let users = repo().await;
        users.join("1", "alice", "Alice").await?;

        assert!(users.record_sync("1", "alice", "Alice").await?);
        assert!(users.due_for_sync("", 10).await?.is_empty());

        users.leave("1", "alice", "Alice").await?;
        assert!(!users.record_sync("1", "late", "Late").await?);

        let left = users.list(1, 20, None, Some("left")).await?;
        assert_eq!(left.users[0].user_name, "alice");
        Ok(())
    }

    #[tokio::test]
    async fn sync_claims_are_exclusive_and_resume_from_the_cursor() -> Result<(), StoreError> {
        let users = repo().await;

        assert_eq!(users.claim_sync().await?, Some(String::new()));
        assert_eq!(users.claim_sync().await?, None);

        users.save_sync_cursor("7").await?;
        users.release_sync().await?;
        assert_eq!(users.claim_sync().await?, Some("7".to_owned()));

        users.finish_sync().await?;
        assert_eq!(users.claim_sync().await?, Some(String::new()));
        Ok(())
    }
}
