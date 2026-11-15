//! Serenity event handling for the bot.

use std::sync::Arc;

use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::{GuildChannel, Message};
use serenity::model::gateway::Ready;
use serenity::model::guild::{Guild, Member, PartialGuild, UnavailableGuild};
use serenity::model::id::GuildId;
use serenity::model::user::User;
use serenity::prelude::{Context, EventHandler};

use crate::commands;
use crate::state;
use crate::{channels, guilds, members};

pub use crate::commands::is_ping;
pub use crate::state::{ChannelsKey, ConfigKey, DatabaseKey, GuildIdKey, GuildsKey};

/// Serenity event handler for the bot.
#[non_exhaustive]
pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _ready: Ready) {
        if let Err(error) = commands::register(&ctx, state::target_guild(&ctx).await).await {
            tracing::error!("Failed to register slash commands: {error}");
        }

        if let (Some(database), Some(guild_id)) =
            (state::database(&ctx).await, state::target_guild(&ctx).await)
        {
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

        if let Some(sender) = state::guild_sender(&ctx).await {
            let target = state::target_guild(&ctx).await;
            sender.send_replace(guilds::snapshot(&ctx.cache, target));
        }
        channels::publish(&ctx).await;
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        if !state::is_target_guild(&ctx, guild.id).await {
            return;
        }

        let details = guilds::details(&guild);
        guilds::publish(&ctx, |entries| guilds::upsert(entries, details)).await;
        channels::publish(&ctx).await;
    }

    async fn guild_update(&self, ctx: Context, _old: Option<Guild>, new: PartialGuild) {
        if !state::is_target_guild(&ctx, new.id).await {
            return;
        }

        let details = guilds::partial_details(&new);
        guilds::publish(&ctx, |entries| {
            let existing = entries.iter().find(|guild| guild.id == details.id);
            let details = guilds::merge_update(existing, details);
            guilds::upsert(entries, details)
        })
        .await;
    }

    async fn guild_delete(&self, ctx: Context, incomplete: UnavailableGuild, _full: Option<Guild>) {
        if !guilds::is_removal(incomplete.unavailable)
            || !state::is_target_guild(&ctx, incomplete.id).await
        {
            return;
        }

        let id = incomplete.id.to_string();
        guilds::publish(&ctx, |guilds| guilds::remove(guilds, &id)).await;
        channels::publish(&ctx).await;
    }

    async fn channel_create(&self, ctx: Context, _channel: GuildChannel) {
        channels::publish(&ctx).await;
    }

    async fn channel_update(&self, ctx: Context, _old: Option<GuildChannel>, _new: GuildChannel) {
        channels::publish(&ctx).await;
    }

    async fn channel_delete(
        &self,
        ctx: Context,
        _channel: GuildChannel,
        _messages: Option<Vec<Message>>,
    ) {
        channels::publish(&ctx).await;
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        if !state::is_target_guild(&ctx, new_member.guild_id).await {
            return;
        }

        let Some(database) = state::database(&ctx).await else {
            return;
        };

        let user = &new_member.user;
        let display_name = members::display_name(
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
        if !state::is_target_guild(&ctx, guild_id).await {
            return;
        }

        let Some(database) = state::database(&ctx).await else {
            return;
        };

        let display_name = members::display_name(
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
        if !commands::is_ping(
            msg.content.as_str(),
            /* author_is_bot= */ msg.author.bot,
        ) {
            return;
        }

        if state::config(&ctx).await.is_some() {
            tracing::info!("Responding to ping!");
        }

        if let Err(error) = msg.channel_id.say(&ctx.http, "Pong!").await {
            tracing::error!("Failed to send message: {error}");
        }
    }
}
