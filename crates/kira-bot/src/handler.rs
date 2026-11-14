//! Discord event handling for the `!ping` command.

use std::sync::Arc;

use kira_config::DiscordConfig;
use kira_protocol::{ChannelDetails, GuildDetails};
use kira_store::Database;
use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::{GuildChannel, Message};
use serenity::model::gateway::Ready;
use serenity::model::guild::{Guild, Member, PartialGuild, UnavailableGuild};
use serenity::model::id::GuildId;
use serenity::model::user::User;
use serenity::prelude::{Context, EventHandler, TypeMapKey};
use tokio::sync::watch;

use crate::commands;

/// Type-map key holding the shared Discord configuration.
#[non_exhaustive]
pub struct ConfigKey;

impl TypeMapKey for ConfigKey {
    type Value = Arc<DiscordConfig>;
}

/// Type-map key holding the guild the API serves, when one is configured.
#[non_exhaustive]
pub struct GuildIdKey;

impl TypeMapKey for GuildIdKey {
    type Value = Option<u64>;
}

/// Type-map key holding the sender for the guild list shared with the API server.
#[non_exhaustive]
pub struct GuildsKey;

impl TypeMapKey for GuildsKey {
    type Value = watch::Sender<Vec<GuildDetails>>;
}

/// Type-map key holding the sender for the channel list shared with the API server.
#[non_exhaustive]
pub struct ChannelsKey;

impl TypeMapKey for ChannelsKey {
    type Value = watch::Sender<Vec<ChannelDetails>>;
}

/// Type-map key holding database storage shared with the API server.
#[non_exhaustive]
pub struct DatabaseKey;

impl TypeMapKey for DatabaseKey {
    type Value = Database;
}

/// Serenity event handler for the bot.
#[non_exhaustive]
pub struct Handler;

/// Return whether a message is a `!ping` command from a human.
#[must_use]
pub fn is_ping(content: &str, author_is_bot: bool) -> bool {
    !author_is_bot && content == "!ping"
}

fn sort_guilds(guilds: &mut [GuildDetails]) {
    guilds.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
}

fn upsert_guild(guilds: &mut Vec<GuildDetails>, guild: GuildDetails) -> bool {
    match guilds.iter_mut().find(|existing| existing.id == guild.id) {
        Some(existing) if *existing == guild => false,
        Some(existing) => {
            *existing = guild;
            sort_guilds(guilds);
            true
        }
        None => {
            guilds.push(guild);
            sort_guilds(guilds);
            true
        }
    }
}

fn remove_guild(guilds: &mut Vec<GuildDetails>, id: &str) -> bool {
    let previous_len = guilds.len();
    guilds.retain(|guild| guild.id != id);
    guilds.len() != previous_len
}

fn guild_details(guild: &Guild) -> GuildDetails {
    let mut details = GuildDetails::new(guild.id.to_string(), guild.name.clone());
    details.description = guild.description.clone();
    details.icon_url = guild.icon_url();
    details.owner_id = guild.owner_id.to_string();
    details.member_count = guild.member_count;
    details.presence_count = guild.approximate_presence_count;
    details.preferred_locale = guild.preferred_locale.clone();
    details.features = guild.features.clone();
    details
}

fn partial_guild_details(guild: &PartialGuild) -> GuildDetails {
    let mut details = GuildDetails::new(guild.id.to_string(), guild.name.clone());
    details.description = guild.description.clone();
    details.icon_url = guild.icon_url();
    details.owner_id = guild.owner_id.to_string();
    details.member_count = guild.approximate_member_count.unwrap_or_default();
    details.presence_count = guild.approximate_presence_count;
    details.preferred_locale = guild.preferred_locale.clone();
    details.features = guild.features.clone();
    details
}

fn merge_guild_update(existing: Option<&GuildDetails>, mut details: GuildDetails) -> GuildDetails {
    if let Some(existing) = existing {
        details.member_count = existing.member_count;

        if details.presence_count.is_none() {
            details.presence_count = existing.presence_count;
        }
    }

    details
}

fn is_guild_removal(unavailable: bool) -> bool {
    !unavailable
}

fn guild_snapshot(cache: &serenity::cache::Cache, target: Option<u64>) -> Vec<GuildDetails> {
    let mut guilds = cache
        .guilds()
        .into_iter()
        .filter(|id| target.is_none_or(|target| id.get() == target))
        .filter_map(|id| cache.guild(id))
        .map(|guild| guild_details(&guild))
        .collect::<Vec<_>>();

    sort_guilds(&mut guilds);
    guilds
}

async fn target_guild(ctx: &Context) -> Option<u64> {
    let data = ctx.data.read().await;
    data.get::<GuildIdKey>().and_then(|id| *id)
}

async fn is_target_guild(ctx: &Context, id: GuildId) -> bool {
    target_guild(ctx)
        .await
        .is_none_or(|target| id.get() == target)
}

async fn guild_sender(ctx: &Context) -> Option<watch::Sender<Vec<GuildDetails>>> {
    let data = ctx.data.read().await;
    data.get::<GuildsKey>().cloned()
}

async fn publish_guilds(ctx: &Context, update: impl FnOnce(&mut Vec<GuildDetails>) -> bool) {
    let Some(sender) = guild_sender(ctx).await else {
        return;
    };

    sender.send_if_modified(update);
}

fn channel_details(channel: &GuildChannel) -> ChannelDetails {
    let mut details = ChannelDetails::new(
        channel.id.to_string(),
        channel.guild_id.to_string(),
        channel.name.clone(),
        channel.kind.name(),
    );
    details.position = channel.position;
    details.parent_id = channel.parent_id.map(|id| id.to_string());
    details
}

fn sort_channels(channels: &mut [ChannelDetails]) {
    channels.sort_by(|a, b| {
        a.guild_id
            .cmp(&b.guild_id)
            .then(a.position.cmp(&b.position))
            .then_with(|| a.name.cmp(&b.name))
    });
}

fn channel_snapshot(cache: &serenity::cache::Cache, target: Option<u64>) -> Vec<ChannelDetails> {
    let mut channels = Vec::new();

    for id in cache.guilds() {
        if target.is_some_and(|target| id.get() != target) {
            continue;
        }

        if let Some(guild) = cache.guild(id) {
            channels.extend(guild.channels.values().map(channel_details));
        }
    }

    sort_channels(&mut channels);
    channels
}

async fn publish_channels(ctx: &Context) {
    let sender = {
        let data = ctx.data.read().await;
        data.get::<ChannelsKey>().cloned()
    };

    if let Some(sender) = sender {
        let target = target_guild(ctx).await;
        sender.send_replace(channel_snapshot(&ctx.cache, target));
    }
}

async fn database(ctx: &Context) -> Option<Database> {
    let data = ctx.data.read().await;
    data.get::<DatabaseKey>().cloned()
}

/// Prefer the guild nickname, then the global display name, then the username.
pub(crate) fn member_display_name(
    nick: Option<&str>,
    global_name: Option<&str>,
    user_name: &str,
) -> String {
    nick.or(global_name).unwrap_or(user_name).to_owned()
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _ready: Ready) {
        if let Err(error) = commands::register(&ctx, target_guild(&ctx).await).await {
            tracing::error!("Failed to register slash commands: {error}");
        }

        if let (Some(database), Some(guild_id)) = (database(&ctx).await, target_guild(&ctx).await) {
            let result = crate::sync::reconcile(
                Arc::clone(&ctx.http),
                GuildId::new(guild_id),
                &database.users(),
            )
            .await;

            if let Err(error) = result {
                tracing::error!("Failed to reconcile guild members: {error}");
            }
        }

        if let Some(sender) = guild_sender(&ctx).await {
            let target = target_guild(&ctx).await;
            sender.send_replace(guild_snapshot(&ctx.cache, target));
        }
        publish_channels(&ctx).await;
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        if !is_target_guild(&ctx, guild.id).await {
            return;
        }

        let details = guild_details(&guild);
        publish_guilds(&ctx, |guilds| upsert_guild(guilds, details)).await;
        publish_channels(&ctx).await;
    }

    async fn guild_update(&self, ctx: Context, _old: Option<Guild>, new: PartialGuild) {
        if !is_target_guild(&ctx, new.id).await {
            return;
        }

        let details = partial_guild_details(&new);
        publish_guilds(&ctx, |guilds| {
            let existing = guilds.iter().find(|guild| guild.id == details.id);
            let details = merge_guild_update(existing, details);
            upsert_guild(guilds, details)
        })
        .await;
    }

    async fn guild_delete(&self, ctx: Context, incomplete: UnavailableGuild, _full: Option<Guild>) {
        if !is_guild_removal(incomplete.unavailable) || !is_target_guild(&ctx, incomplete.id).await
        {
            return;
        }

        let id = incomplete.id.to_string();
        publish_guilds(&ctx, |guilds| remove_guild(guilds, &id)).await;
        publish_channels(&ctx).await;
    }

    async fn channel_create(&self, ctx: Context, _channel: GuildChannel) {
        publish_channels(&ctx).await;
    }

    async fn channel_update(&self, ctx: Context, _old: Option<GuildChannel>, _new: GuildChannel) {
        publish_channels(&ctx).await;
    }

    async fn channel_delete(
        &self,
        ctx: Context,
        _channel: GuildChannel,
        _messages: Option<Vec<Message>>,
    ) {
        publish_channels(&ctx).await;
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        if !is_target_guild(&ctx, new_member.guild_id).await {
            return;
        }

        let Some(database) = database(&ctx).await else {
            return;
        };

        let user = &new_member.user;
        let display_name = member_display_name(
            new_member.nick.as_deref(),
            user.global_name.as_deref(),
            &user.name,
        );

        if let Err(error) = database
            .users()
            .join(&user.id.to_string(), &user.name, &display_name)
            .await
        {
            tracing::error!("Failed to record guild join: {error:?}");
        }
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        member_data_if_available: Option<Member>,
    ) {
        if !is_target_guild(&ctx, guild_id).await {
            return;
        }

        let Some(database) = database(&ctx).await else {
            return;
        };

        let display_name = member_display_name(
            member_data_if_available
                .as_ref()
                .and_then(|member| member.nick.as_deref()),
            user.global_name.as_deref(),
            &user.name,
        );

        if let Err(error) = database
            .users()
            .leave(&user.id.to_string(), &user.name, &display_name)
            .await
        {
            tracing::error!("Failed to record guild leave: {error:?}");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let result = match interaction {
            Interaction::Command(command) => commands::handle(&ctx, &command).await,
            Interaction::Modal(modal) => commands::handle_modal(&ctx, &modal).await,
            _ => Ok(()),
        };

        if let Err(error) = result {
            tracing::error!("Failed to respond to interaction: {error}");
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if !is_ping(
            msg.content.as_str(),
            /* author_is_bot= */ msg.author.bot,
        ) {
            return;
        }

        // Access shared configuration.
        let config = {
            let data = ctx.data.read().await;
            data.get::<ConfigKey>().cloned()
        };

        if config.is_some() {
            tracing::info!("Responding to ping!");
        }

        if let Err(error) = msg.channel_id.say(&ctx.http, "Pong!").await {
            tracing::error!("Failed to send message: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use kira_protocol::{ChannelDetails, GuildDetails};

    use super::{
        is_guild_removal, is_ping, merge_guild_update, remove_guild, sort_channels, upsert_guild,
    };

    #[test]
    fn ping_from_human_matches() {
        assert!(is_ping("!ping", false));
    }

    #[test]
    fn ping_from_bot_is_ignored() {
        assert!(!is_ping("!ping", true));
    }

    #[test]
    fn other_text_is_ignored() {
        assert!(!is_ping("hello", false));
    }

    #[test]
    fn upsert_guild_adds_in_name_order() {
        let mut guilds = vec![GuildDetails::new("1", "Beta")];

        assert!(upsert_guild(&mut guilds, GuildDetails::new("2", "Alpha")));
        assert_eq!(
            guilds,
            vec![
                GuildDetails::new("2", "Alpha"),
                GuildDetails::new("1", "Beta"),
            ]
        );
    }

    #[test]
    fn upsert_guild_replaces_the_existing_entry() {
        let mut guilds = vec![GuildDetails::new("1", "Old")];

        assert!(upsert_guild(&mut guilds, GuildDetails::new("1", "New")));
        assert_eq!(guilds, vec![GuildDetails::new("1", "New")]);
    }

    #[test]
    fn upsert_guild_reports_an_unchanged_entry() {
        let mut guilds = vec![GuildDetails::new("1", "Same")];

        assert!(!upsert_guild(&mut guilds, GuildDetails::new("1", "Same")));
        assert_eq!(guilds, vec![GuildDetails::new("1", "Same")]);
    }

    #[test]
    fn remove_guild_deletes_by_id() {
        let mut guilds = vec![
            GuildDetails::new("1", "Alpha"),
            GuildDetails::new("2", "Beta"),
        ];

        assert!(remove_guild(&mut guilds, "1"));
        assert_eq!(guilds, vec![GuildDetails::new("2", "Beta")]);
        assert!(!remove_guild(&mut guilds, "9"));
    }

    #[test]
    fn guild_update_preserves_cached_counts() {
        let mut existing = GuildDetails::new("1", "Alpha");
        existing.member_count = 12;
        existing.presence_count = Some(3);

        let updated = GuildDetails::new("1", "Renamed");

        let merged = merge_guild_update(Some(&existing), updated);

        assert_eq!(merged.name, "Renamed");
        assert_eq!(merged.member_count, 12);
        assert_eq!(merged.presence_count, Some(3));
    }

    #[test]
    fn guild_update_keeps_a_reported_presence_count() {
        let mut existing = GuildDetails::new("1", "Alpha");
        existing.presence_count = Some(2);

        let mut updated = GuildDetails::new("1", "Alpha");
        updated.presence_count = Some(5);

        let merged = merge_guild_update(Some(&existing), updated);
        assert_eq!(merged.presence_count, Some(5));
    }

    #[test]
    fn channels_are_sorted_by_guild_position_and_name() {
        let mut late = ChannelDetails::new("3", "2", "general", "text");
        late.position = 1;
        let mut early = ChannelDetails::new("2", "1", "voice", "voice");
        early.position = 5;
        let first = ChannelDetails::new("1", "1", "general", "text");

        let mut channels = vec![late, early, first.clone()];
        sort_channels(&mut channels);

        assert_eq!(first, channels[0]);
        assert_eq!("2", channels[1].id);
        assert_eq!("3", channels[2].id);
    }

    #[test]
    fn unavailable_guild_events_are_not_removals() {
        assert!(!is_guild_removal(true));
        assert!(is_guild_removal(false));
    }
}
