//! Discord event handling for the `!ping` command.

use std::sync::Arc;

use kira_config::DiscordConfig;
use kira_protocol::GuildDetails;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::guild::{Guild, PartialGuild, UnavailableGuild};
use serenity::prelude::{Context, EventHandler, TypeMapKey};
use tokio::sync::watch;

/// Type-map key holding the shared Discord configuration.
#[non_exhaustive]
pub struct ConfigKey;

impl TypeMapKey for ConfigKey {
    type Value = Arc<DiscordConfig>;
}

/// Type-map key holding the sender for the guild list shared with the API server.
#[non_exhaustive]
pub struct GuildsKey;

impl TypeMapKey for GuildsKey {
    type Value = watch::Sender<Vec<GuildDetails>>;
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

fn guild_snapshot(cache: &serenity::cache::Cache) -> Vec<GuildDetails> {
    let mut guilds = cache
        .guilds()
        .into_iter()
        .filter_map(|id| cache.guild(id))
        .map(|guild| guild_details(&guild))
        .collect::<Vec<_>>();

    sort_guilds(&mut guilds);
    guilds
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

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _ready: Ready) {
        if let Some(sender) = guild_sender(&ctx).await {
            sender.send_replace(guild_snapshot(&ctx.cache));
        }
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        let details = guild_details(&guild);
        publish_guilds(&ctx, |guilds| upsert_guild(guilds, details)).await;
    }

    async fn guild_update(&self, ctx: Context, _old: Option<Guild>, new: PartialGuild) {
        let mut details = partial_guild_details(&new);
        publish_guilds(&ctx, |guilds| {
            if let Some(existing) = guilds.iter().find(|guild| guild.id == details.id) {
                details.member_count = existing.member_count;
            }
            upsert_guild(guilds, details)
        })
        .await;
    }

    async fn guild_delete(&self, ctx: Context, incomplete: UnavailableGuild, _full: Option<Guild>) {
        let id = incomplete.id.to_string();
        publish_guilds(&ctx, |guilds| remove_guild(guilds, &id)).await;
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
            println!("Responding to ping!");
        }

        if let Err(error) = msg.channel_id.say(&ctx.http, "Pong!").await {
            eprintln!("Failed to send message: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use kira_protocol::GuildDetails;

    use super::{is_ping, remove_guild, upsert_guild};

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
}
