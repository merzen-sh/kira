//! Shared Serenity state and accessors.

use std::sync::Arc;

use kira_config::DiscordConfig;
use kira_protocol::{ChannelDetails, GuildDetails};
use kira_store::Database;
use serenity::model::id::GuildId;
use serenity::prelude::{Context, TypeMapKey};
use tokio::sync::watch;

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

pub(crate) async fn config(ctx: &Context) -> Option<Arc<DiscordConfig>> {
    let data = ctx.data.read().await;
    data.get::<ConfigKey>().cloned()
}

pub(crate) async fn target_guild(ctx: &Context) -> Option<u64> {
    let data = ctx.data.read().await;
    data.get::<GuildIdKey>().and_then(|id| *id)
}

pub(crate) async fn is_target_guild(ctx: &Context, id: GuildId) -> bool {
    target_guild(ctx)
        .await
        .is_none_or(|target| id.get() == target)
}

pub(crate) async fn guild_sender(ctx: &Context) -> Option<watch::Sender<Vec<GuildDetails>>> {
    let data = ctx.data.read().await;
    data.get::<GuildsKey>().cloned()
}

pub(crate) async fn database(ctx: &Context) -> Option<Database> {
    let data = ctx.data.read().await;
    data.get::<DatabaseKey>().cloned()
}
