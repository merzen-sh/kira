//! Periodic metadata refresh for tracked guild members.

use std::sync::Arc;
use std::time::Duration;

use kira_store::StoreError;
use kira_store::users::UserRepository;
use serenity::http::Http;
use serenity::model::id::{GuildId, UserId};

/// How often the worker looks for users due for a refresh.
const SYNC_INTERVAL: Duration = Duration::from_secs(15 * 60);

/// Users fetched per batch.
const BATCH_SIZE: u32 = 50;

/// Discord requests attempted per user.
const MAX_ATTEMPTS: usize = 3;

/// Delay between retries for a failed Discord request.
const RETRY_DELAY: Duration = Duration::from_secs(2);

pub(crate) async fn run(http: Arc<Http>, guild_id: GuildId, users: UserRepository) {
    loop {
        match sync_once(&http, guild_id, &users).await {
            Ok(()) => tracing::debug!("Guild user metadata sync finished."),
            Err(error) => tracing::error!("Guild user metadata sync failed: {error:?}"),
        }

        tokio::time::sleep(SYNC_INTERVAL).await;
    }
}

async fn sync_once(
    http: &Http,
    guild_id: GuildId,
    users: &UserRepository,
) -> Result<(), StoreError> {
    let Some(mut cursor) = users.claim_sync().await? else {
        return Ok(());
    };

    match sync_batches(http, guild_id, users, &mut cursor).await {
        Ok(()) => users.finish_sync().await,
        Err(error) => {
            let _ = users.release_sync().await;
            Err(error)
        }
    }
}

async fn sync_batches(
    http: &Http,
    guild_id: GuildId,
    users: &UserRepository,
    cursor: &mut String,
) -> Result<(), StoreError> {
    loop {
        let batch = users.due_for_sync(cursor, BATCH_SIZE).await?;
        if batch.is_empty() {
            return Ok(());
        }

        for user_id in batch {
            if let Some((user_name, display_name)) = fetch_member(http, guild_id, &user_id).await {
                users
                    .record_sync(&user_id, &user_name, &display_name)
                    .await?;
            }

            // Advance even after a failed fetch so one user cannot block the
            // batch; the unchanged cooldown makes it due again next run.
            *cursor = user_id;
            users.save_sync_cursor(cursor).await?;
        }
    }
}

async fn fetch_member(http: &Http, guild_id: GuildId, user_id: &str) -> Option<(String, String)> {
    let Ok(id) = user_id.parse::<u64>() else {
        tracing::warn!("Skipping guild user with invalid ID: {user_id}");
        return None;
    };

    for attempt in 1..=MAX_ATTEMPTS {
        match guild_id.member(http, UserId::new(id)).await {
            Ok(member) => {
                let user = &member.user;
                let display_name = member
                    .nick
                    .or_else(|| user.global_name.clone())
                    .unwrap_or_else(|| user.name.clone());
                return Some((user.name.clone(), display_name));
            }
            Err(error) if attempt < MAX_ATTEMPTS => {
                tracing::warn!("Fetching guild user {user_id} failed (attempt {attempt}): {error}");
                tokio::time::sleep(RETRY_DELAY).await;
            }
            Err(error) => {
                tracing::error!("Fetching guild user {user_id} failed: {error}");
                return None;
            }
        }
    }

    None
}
