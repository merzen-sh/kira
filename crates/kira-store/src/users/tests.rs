//! Guild user persistence tests.

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

    users.join("1", "alice", "Alice", false).await?;

    let page = users.list(1, 20, None, None, None).await?;
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

    users.join("1", "alice", "Alice", false).await?;
    users.leave("1", "alice", "Alice", false).await?;

    let left = users.list(1, 20, None, Some("left"), None).await?;
    assert_eq!(left.total, 1);
    assert!(left.users[0].leave_at.is_some());

    users.join("1", "alice2", "Alice Two", false).await?;

    let active = users.list(1, 20, None, Some("active"), None).await?;
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
    users.join("1", "alice", "Alice", false).await?;
    users.join("2", "bob", "Bob", false).await?;
    users.leave("2", "bob", "Bob", false).await?;

    let all = users.list(1, 20, None, None, None).await?;
    assert_eq!(all.total, 2);

    let searched = users.list(1, 20, Some("bob"), None, None).await?;
    assert_eq!(searched.total, 1);
    assert_eq!(searched.users[0].user_id, "2");

    let active = users.list(1, 20, None, Some("active"), None).await?;
    assert_eq!(active.total, 1);
    assert_eq!(active.users[0].user_id, "1");

    let percent = users.list(1, 20, Some("%"), None, None).await?;
    assert_eq!(percent.total, 0);

    let second = users.list(2, 1, None, None, None).await?;
    assert_eq!(second.total, 2);
    assert_eq!(second.users.len(), 1);
    Ok(())
}

#[tokio::test]
async fn stats_count_active_and_left_users() -> Result<(), StoreError> {
    let users = repo().await;
    users.join("1", "alice", "Alice", false).await?;
    users.join("2", "bob", "Bob", false).await?;
    users.join("3", "kira", "Kira", true).await?;
    users.leave("2", "bob", "Bob", false).await?;

    let stats = users.stats().await?;
    assert_eq!(
        (stats.total, stats.active, stats.left, stats.bots),
        (2, 1, 1, 1)
    );
    Ok(())
}

#[tokio::test]
async fn reconciliation_updates_the_bot_flag() -> Result<(), StoreError> {
    let users = repo().await;
    users.join("1", "kira", "Kira", false).await?;
    users.join("2", "alice", "Alice", false).await?;

    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let snapshot = users.snapshot_time().await?;

    users
        .import_member("1", "kira", "Kira", true, None, &snapshot)
        .await?;

    let bots = users.list(1, 20, None, None, Some(true)).await?;
    assert_eq!(bots.total, 1);
    assert_eq!(bots.users[0].user_id, "1");
    assert!(bots.users[0].is_bot);

    let humans = users.list(1, 20, None, None, Some(false)).await?;
    assert_eq!(humans.total, 1);
    assert_eq!(humans.users[0].user_id, "2");
    Ok(())
}

#[tokio::test]
async fn leave_records_the_bot_flag() -> Result<(), StoreError> {
    let users = repo().await;
    users.join("1", "kira", "Kira", false).await?;
    users.leave("1", "kira", "Kira", true).await?;

    let bots = users.list(1, 20, None, Some("left"), Some(true)).await?;
    assert_eq!(bots.total, 1);
    assert_eq!(bots.users[0].status, "left");
    assert!(bots.users[0].is_bot);

    let stats = users.stats().await?;
    assert_eq!(
        (stats.total, stats.active, stats.left, stats.bots),
        (0, 0, 0, 1)
    );
    Ok(())
}

#[tokio::test]
async fn reconciliation_imports_reactivates_and_marks_missing_left() -> Result<(), StoreError> {
    let users = repo().await;
    users.join("1", "alice", "Alice", false).await?;
    users.join("2", "bob", "Bob", false).await?;
    users.leave("2", "bob", "Bob", false).await?;

    // Ensure the seeded joins predate the reconciliation snapshot.
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let snapshot = users.snapshot_time().await?;

    users
        .import_member("1", "alice", "Alice", false, None, &snapshot)
        .await?;
    users
        .import_member(
            "2",
            "bob",
            "Bob",
            false,
            Some("2021-02-03T04:05:06.000Z"),
            &snapshot,
        )
        .await?;

    // Re-importing an active user does not duplicate Join history.
    assert_eq!(event_count(&users, "1").await?, 1);
    // Reactivating a left user appends a Join event and uses Discord's date.
    assert_eq!(event_count(&users, "2").await?, 3);
    let active = users.list(1, 20, None, Some("active"), None).await?;
    let bob = active.users.iter().find(|user| user.user_id == "2");
    assert_eq!(
        bob.map(|user| user.join_at.as_str()),
        Some("2021-02-03T04:05:06.000Z")
    );

    users.mark_left(&["1".to_owned()], &snapshot).await?;

    let left = users.list(1, 20, None, Some("left"), None).await?;
    assert_eq!(left.total, 1);
    assert_eq!(left.users[0].user_id, "1");
    assert_eq!(event_count(&users, "1").await?, 2);
    assert_eq!(users.active_ids().await?, vec!["2"]);
    Ok(())
}

#[tokio::test]
async fn imported_members_use_discord_join_time() -> Result<(), StoreError> {
    let users = repo().await;
    let snapshot = users.snapshot_time().await?;

    users
        .import_member(
            "9",
            "new",
            "New",
            false,
            Some("2020-05-06T07:08:09.000Z"),
            &snapshot,
        )
        .await?;

    let page = users.list(1, 20, None, None, None).await?;
    assert_eq!(page.users[0].join_at, "2020-05-06T07:08:09.000Z");
    Ok(())
}

#[tokio::test]
async fn reconciliation_does_not_undo_events_after_its_snapshot() -> Result<(), StoreError> {
    let users = repo().await;
    let snapshot = users.snapshot_time().await?;

    // A Join after the snapshot must not be overwritten by mark_left.
    users.join("1", "alice", "Alice", false).await?;
    users.mark_left(&["1".to_owned()], &snapshot).await?;
    assert_eq!(users.active_ids().await?, vec!["1"]);

    // A Leave after the snapshot must not be undone by import_member.
    let snapshot = users.snapshot_time().await?;
    users.leave("1", "alice", "Alice", false).await?;
    users
        .import_member("1", "alice", "Alice", false, None, &snapshot)
        .await?;

    let left = users.list(1, 20, None, Some("left"), None).await?;
    assert_eq!(left.total, 1);
    Ok(())
}

#[tokio::test]
async fn sync_only_selects_active_users_due_after_the_cursor() -> Result<(), StoreError> {
    let users = repo().await;
    users.join("1", "alice", "Alice", false).await?;
    users.join("2", "bob", "Bob", false).await?;
    users.leave("2", "bob", "Bob", false).await?;

    let due = users.due_for_sync("", 10).await?;
    assert_eq!(due, vec!["1"]);

    assert!(users.due_for_sync("1", 10).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn successful_sync_sets_the_cooldown_and_respects_a_leave() -> Result<(), StoreError> {
    let users = repo().await;
    users.join("1", "alice", "Alice", false).await?;

    assert!(users.record_sync("1", "alice", "Alice").await?);
    assert!(users.due_for_sync("", 10).await?.is_empty());

    users.leave("1", "alice", "Alice", false).await?;
    assert!(!users.record_sync("1", "late", "Late").await?);

    let left = users.list(1, 20, None, Some("left"), None).await?;
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
