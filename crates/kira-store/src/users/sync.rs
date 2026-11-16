//! Guild user metadata sync bookkeeping.

use super::{NEXT_SYNC, NOW, STALE_LOCK, UserRepository};
use crate::StoreError;

impl UserRepository {
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
        avatar_url: Option<&str>,
    ) -> Result<bool, StoreError> {
        let sql = format!(
            "UPDATE guild_users \
             SET user_name = ?, display_name = ?, avatar_url = COALESCE(?, avatar_url), \
                 last_synced_at = {NOW}, next_sync_at = {NEXT_SYNC} \
             WHERE user_id = ? AND status = 'active'"
        );

        let result = sqlx::query(&sql)
            .bind(user_name)
            .bind(display_name)
            .bind(avatar_url)
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
