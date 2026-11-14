//! Guild member reconciliation and periodic metadata refresh.

use std::collections::HashSet;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use kira_store::StoreError;
use kira_store::users::UserRepository;
use serenity::http::{Http, StatusCode};
use serenity::model::id::{GuildId, UserId};

use crate::handler::member_display_name;

/// How often the worker looks for users due for a refresh.
const SYNC_INTERVAL: Duration = Duration::from_secs(15 * 60);

/// Users fetched per sync batch.
const BATCH_SIZE: u32 = 50;

/// Members fetched per reconciliation page.
const RECONCILE_PAGE: u64 = 1000;

/// Discord requests attempted per user.
const MAX_ATTEMPTS: usize = 3;

/// Delay between retries for a failed Discord request.
const RETRY_DELAY: Duration = Duration::from_secs(2);

/// Refresh metadata for due users forever.
pub(crate) async fn run(http: Arc<Http>, guild_id: GuildId, users: UserRepository) {
    let source = DiscordMembers { http, guild_id };

    loop {
        match sync_once(&users, &source).await {
            Ok(()) => tracing::debug!("Guild user metadata sync finished."),
            Err(error) => tracing::error!("Guild user metadata sync failed: {error:?}"),
        }

        tokio::time::sleep(SYNC_INTERVAL).await;
    }
}

/// Import the current guild membership and mark missing active users as left.
///
/// Runs on every `ready` so a cold start or an outage does not leave the user
/// table stale. Only Discord membership determines status; the sync worker
/// does not.
pub(crate) async fn reconcile(
    http: Arc<Http>,
    guild_id: GuildId,
    users: &UserRepository,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut after = None;
    let mut seen = HashSet::new();

    loop {
        let members = guild_id.members(&http, Some(RECONCILE_PAGE), after).await?;
        let fetched = members.len();

        for member in members {
            let id = member.user.id;
            let display_name = member_display_name(
                member.nick.as_deref(),
                member.user.global_name.as_deref(),
                &member.user.name,
            );

            users
                .import_member(&id.to_string(), &member.user.name, &display_name)
                .await?;
            seen.insert(id.to_string());
            after = Some(id);
        }

        if fetched < RECONCILE_PAGE as usize {
            break;
        }
    }

    let missing: Vec<String> = users
        .active_ids()
        .await?
        .into_iter()
        .filter(|user_id| !seen.contains(user_id))
        .collect();
    users.mark_left(&missing).await?;

    Ok(())
}

async fn sync_once<S: MemberSource>(users: &UserRepository, source: &S) -> Result<(), StoreError> {
    let Some(mut cursor) = users.claim_sync().await? else {
        return Ok(());
    };

    match sync_batches(users, source, &mut cursor).await {
        Ok(()) => users.finish_sync().await,
        Err(error) => {
            let _ = users.release_sync().await;
            Err(error)
        }
    }
}

async fn sync_batches<S: MemberSource>(
    users: &UserRepository,
    source: &S,
    cursor: &mut String,
) -> Result<(), StoreError> {
    loop {
        let batch = users.due_for_sync(cursor, BATCH_SIZE).await?;
        if batch.is_empty() {
            return Ok(());
        }

        for user_id in batch {
            match source.fetch(&user_id).await {
                FetchOutcome::Found {
                    user_name,
                    display_name,
                } => {
                    users
                        .record_sync(&user_id, &user_name, &display_name)
                        .await?;
                }
                FetchOutcome::Missing => {
                    // Do not change membership here; reconciliation owns that.
                    users.defer_sync(&user_id).await?;
                }
                FetchOutcome::Failed => {}
            }

            // Advance even after a failure so one user cannot block the
            // batch; an unchanged cooldown makes it due again next run.
            *cursor = user_id;
            users.save_sync_cursor(cursor).await?;
        }
    }
}

enum FetchOutcome {
    Found {
        user_name: String,
        display_name: String,
    },
    Missing,
    Failed,
}

trait MemberSource {
    async fn fetch(&self, user_id: &str) -> FetchOutcome;
}

struct DiscordMembers {
    http: Arc<Http>,
    guild_id: GuildId,
}

impl MemberSource for DiscordMembers {
    async fn fetch(&self, user_id: &str) -> FetchOutcome {
        fetch_member(&self.http, self.guild_id, user_id).await
    }
}

async fn fetch_member(http: &Http, guild_id: GuildId, user_id: &str) -> FetchOutcome {
    let Ok(id) = user_id.parse::<u64>() else {
        tracing::warn!("Skipping guild user with invalid ID: {user_id}");
        return FetchOutcome::Missing;
    };

    for attempt in 1..=MAX_ATTEMPTS {
        match guild_id.member(http, UserId::new(id)).await {
            Ok(member) => {
                let user = &member.user;
                let display_name = member
                    .nick
                    .or_else(|| user.global_name.clone())
                    .unwrap_or_else(|| user.name.clone());
                return FetchOutcome::Found {
                    user_name: user.name.clone(),
                    display_name,
                };
            }
            Err(serenity::Error::Http(error))
                if error.status_code() == Some(StatusCode::NOT_FOUND) =>
            {
                // Not a guild member anymore; reconciliation decides status.
                return FetchOutcome::Missing;
            }
            Err(error) if attempt < MAX_ATTEMPTS => {
                tracing::warn!("Fetching guild user {user_id} failed (attempt {attempt}): {error}");
                tokio::time::sleep(RETRY_DELAY).await;
            }
            Err(error) => {
                tracing::error!("Fetching guild user {user_id} failed: {error}");
                return FetchOutcome::Failed;
            }
        }
    }

    FetchOutcome::Failed
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use kira_store::StoreError;

    use super::{FetchOutcome, MemberSource, sync_once};

    struct FakeSource {
        calls: RefCell<Vec<String>>,
        missing: &'static str,
    }

    impl MemberSource for FakeSource {
        async fn fetch(&self, user_id: &str) -> FetchOutcome {
            self.calls.borrow_mut().push(user_id.to_owned());

            if user_id == self.missing {
                FetchOutcome::Missing
            } else {
                FetchOutcome::Found {
                    user_name: format!("user{user_id}"),
                    display_name: format!("User {user_id}"),
                }
            }
        }
    }

    #[tokio::test]
    async fn a_pass_attempts_every_due_user_and_resets_the_checkpoint() -> Result<(), StoreError> {
        let database = kira_store::Database::connect("sqlite::memory:").await?;
        let users = database.users();
        users.join("1", "a", "A").await?;
        users.join("2", "b", "B").await?;
        users.join("3", "c", "C").await?;

        let source = FakeSource {
            calls: RefCell::new(Vec::new()),
            missing: "2",
        };

        sync_once(&users, &source).await?;

        assert_eq!(*source.calls.borrow(), vec!["1", "2", "3"]);
        assert!(users.due_for_sync("", 10).await?.is_empty());

        // `finish_sync` released the lock and cleared the cursor.
        assert_eq!(users.claim_sync().await?, Some(String::new()));
        Ok(())
    }
}
