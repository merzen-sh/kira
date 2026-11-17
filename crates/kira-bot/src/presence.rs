//! Bot presence mapping and live updates.

use std::sync::Arc;

use kira_protocol::{ActivityKind, BotPresence, BotStatus};
use serenity::client::Context;
use serenity::gateway::{ActivityData, ShardManager};
use serenity::model::user::OnlineStatus;
use tokio::sync::watch;

pub(crate) fn online_status(status: BotStatus) -> OnlineStatus {
    match status {
        BotStatus::Online => OnlineStatus::Online,
        BotStatus::Idle => OnlineStatus::Idle,
        BotStatus::DoNotDisturb => OnlineStatus::DoNotDisturb,
        BotStatus::Invisible => OnlineStatus::Invisible,
        _ => OnlineStatus::Online,
    }
}

pub(crate) fn activity(presence: &BotPresence) -> Option<ActivityData> {
    presence.activity.as_ref().map(|activity| {
        let text = activity.text.as_str();

        match activity.kind {
            ActivityKind::Playing => ActivityData::playing(text),
            ActivityKind::Watching => ActivityData::watching(text),
            ActivityKind::Listening => ActivityData::listening(text),
            ActivityKind::Competing => ActivityData::competing(text),
            ActivityKind::Custom => ActivityData::custom(text),
            _ => ActivityData::playing(text),
        }
    })
}

pub(crate) fn apply(ctx: &Context, presence: &BotPresence) {
    ctx.set_presence(activity(presence), online_status(presence.status));
}

pub(crate) async fn run(
    shard_manager: Arc<ShardManager>,
    mut presence: watch::Receiver<BotPresence>,
) {
    while presence.changed().await.is_ok() {
        let current = presence.borrow_and_update().clone();
        let runners = shard_manager.runners.lock().await;

        for runner in runners.values() {
            runner
                .runner_tx
                .set_presence(activity(&current), online_status(current.status));
        }
    }
}

#[cfg(test)]
mod tests {
    use kira_protocol::{ActivityKind, BotActivity, BotPresence, BotStatus};
    use serenity::model::gateway::ActivityType;
    use serenity::model::user::OnlineStatus;

    use super::{activity, online_status};

    #[test]
    fn status_maps_to_online_status() {
        assert_eq!(online_status(BotStatus::Online), OnlineStatus::Online);
        assert_eq!(online_status(BotStatus::Idle), OnlineStatus::Idle);
        assert_eq!(
            online_status(BotStatus::DoNotDisturb),
            OnlineStatus::DoNotDisturb
        );
        assert_eq!(online_status(BotStatus::Invisible), OnlineStatus::Invisible);
    }

    #[test]
    fn activity_maps_to_discord_activity() {
        let cases = [
            (ActivityKind::Playing, ActivityType::Playing, None),
            (ActivityKind::Watching, ActivityType::Watching, None),
            (ActivityKind::Listening, ActivityType::Listening, None),
            (ActivityKind::Competing, ActivityType::Competing, None),
            (ActivityKind::Custom, ActivityType::Custom, Some("Kira")),
        ];

        for (kind, expected_kind, expected_state) in cases {
            let presence =
                BotPresence::new(BotStatus::Online, Some(BotActivity::new(kind, "Kira")));
            let mapped = activity(&presence).expect("activity");
            assert_eq!(mapped.kind, expected_kind);
            assert_eq!(mapped.state.as_deref(), expected_state);
        }
    }

    #[test]
    fn absent_activity_maps_to_none() {
        assert!(activity(&BotPresence::default()).is_none());
    }
}
