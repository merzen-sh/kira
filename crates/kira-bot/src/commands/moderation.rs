//! The `/mod` slash command group and its server-side permission checks.

use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::http::Http;
use serenity::model::application::{
    CommandData, CommandDataOption, CommandDataOptionValue, CommandInteraction, CommandOptionType,
};
use serenity::model::channel::{GuildChannel, PermissionOverwrite, PermissionOverwriteType};
use serenity::model::id::{GuildId, MessageId, UserId};
use serenity::model::mention::Mentionable;
use serenity::model::permissions::Permissions;
use serenity::prelude::Context;
use thiserror::Error;

use crate::commands::permissions::{CommandPermission, PermissionDenied, PermissionScope};
use crate::errors::BotError;

/// Name of the `/mod` slash command group.
pub(super) const NAME: &str = "mod";

const BAN: CommandPermission = CommandPermission::guild(Permissions::BAN_MEMBERS, "Ban Members");
const KICK: CommandPermission = CommandPermission::guild(Permissions::KICK_MEMBERS, "Kick Members");
const PURGE: CommandPermission =
    CommandPermission::channel(Permissions::MANAGE_MESSAGES, "Manage Messages");
const LOCK: CommandPermission =
    CommandPermission::channel(Permissions::MANAGE_CHANNELS, "Manage Channels");
const UNLOCK: CommandPermission =
    CommandPermission::channel(Permissions::MANAGE_CHANNELS, "Manage Channels");

/// Build the `/mod` command group.
pub(super) fn command() -> CreateCommand {
    CreateCommand::new(NAME)
        .description("Moderate this server.")
        // Discord uses these to hide the command from most members; the
        // authoritative check happens in `run` before every action.
        .default_member_permissions(moderator_permissions())
        .dm_permission(false)
        .add_option(ban_option())
        .add_option(kick_option())
        .add_option(purge_option())
        .add_option(lock_option())
        .add_option(unlock_option())
}

fn moderator_permissions() -> Permissions {
    Permissions::ADMINISTRATOR
        | Permissions::BAN_MEMBERS
        | Permissions::KICK_MEMBERS
        | Permissions::MANAGE_MESSAGES
        | Permissions::MANAGE_CHANNELS
}

fn ban_option() -> CreateCommandOption {
    CreateCommandOption::new(CommandOptionType::SubCommand, "ban", "Ban a member.")
        .add_sub_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Member to ban.")
                .required(true),
        )
        .add_sub_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Reason for the ban.")
                .max_length(512),
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "delete_message_days",
                "Days of the member's messages to delete, from 0 to 7.",
            )
            .min_int_value(0)
            .max_int_value(7),
        )
}

fn kick_option() -> CreateCommandOption {
    CreateCommandOption::new(CommandOptionType::SubCommand, "kick", "Kick a member.")
        .add_sub_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Member to kick.")
                .required(true),
        )
        .add_sub_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Reason for the kick.")
                .max_length(512),
        )
}

fn purge_option() -> CreateCommandOption {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "purge",
        "Delete recent messages in this channel.",
    )
    .add_sub_option(
        CreateCommandOption::new(
            CommandOptionType::Integer,
            "count",
            "Number of messages to delete, from 1 to 100.",
        )
        .required(true)
        .min_int_value(1)
        .max_int_value(100),
    )
}

fn lock_option() -> CreateCommandOption {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "lock",
        "Prevent @everyone from sending messages here.",
    )
}

fn unlock_option() -> CreateCommandOption {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "unlock",
        "Allow @everyone to send messages here again.",
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModerationCommand {
    Ban {
        user: UserId,
        reason: Option<String>,
        delete_message_days: u8,
    },
    Kick {
        user: UserId,
        reason: Option<String>,
    },
    Purge {
        count: u8,
    },
    Lock,
    Unlock,
}

impl ModerationCommand {
    fn parse(data: &CommandData) -> Option<Self> {
        let (name, options) = data.options.iter().find_map(|option| match &option.value {
            CommandDataOptionValue::SubCommand(options) => {
                Some((option.name.as_str(), options.as_slice()))
            }
            _ => None,
        })?;

        match name {
            "ban" => {
                let user = user(options, "user")?;
                let delete_message_days = integer(options, "delete_message_days").unwrap_or(0);
                let delete_message_days = u8::try_from(delete_message_days)
                    .ok()
                    .filter(|days| *days <= 7)?;

                Some(Self::Ban {
                    user,
                    reason: reason(options),
                    delete_message_days,
                })
            }
            "kick" => Some(Self::Kick {
                user: user(options, "user")?,
                reason: reason(options),
            }),
            "purge" => {
                let count = integer(options, "count")?;
                let count = u8::try_from(count)
                    .ok()
                    .filter(|count| (1..=100).contains(count))?;

                Some(Self::Purge { count })
            }
            "lock" => Some(Self::Lock),
            "unlock" => Some(Self::Unlock),
            _ => None,
        }
    }

    const fn permission(&self) -> CommandPermission {
        match self {
            Self::Ban { .. } => BAN,
            Self::Kick { .. } => KICK,
            Self::Purge { .. } => PURGE,
            Self::Lock => LOCK,
            Self::Unlock => UNLOCK,
        }
    }
}

fn user(options: &[CommandDataOption], name: &str) -> Option<UserId> {
    options.iter().find_map(|option| match &option.value {
        CommandDataOptionValue::User(user) if option.name == name => Some(*user),
        _ => None,
    })
}

fn integer(options: &[CommandDataOption], name: &str) -> Option<i64> {
    options.iter().find_map(|option| match &option.value {
        CommandDataOptionValue::Integer(value) if option.name == name => Some(*value),
        _ => None,
    })
}

fn string<'a>(options: &'a [CommandDataOption], name: &str) -> Option<&'a str> {
    options.iter().find_map(|option| match &option.value {
        CommandDataOptionValue::String(value) if option.name == name => Some(value.as_str()),
        _ => None,
    })
}

fn reason(options: &[CommandDataOption]) -> Option<String> {
    string(options, "reason")
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map(str::to_owned)
}

/// Handle a `/mod` interaction and always answer ephemerally.
pub(super) async fn handle(ctx: &Context, command: &CommandInteraction) -> Result<(), BotError> {
    let content = match handle_command(ctx, command).await {
        Ok(content) => content,
        Err(error) => {
            if !matches!(error, ModerationError::PermissionDenied(_)) {
                tracing::warn!(error = %error, "moderation command failed");
            }
            error.to_string()
        }
    };

    respond_ephemeral(ctx, command, content).await
}

async fn handle_command(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<String, ModerationError> {
    let guild_id = command.guild_id.ok_or(ModerationError::MissingGuild)?;
    let parsed = ModerationCommand::parse(&command.data).ok_or(ModerationError::InvalidCommand)?;

    let (channel, permissions) = {
        let guild = ctx
            .cache
            .guild(guild_id)
            .ok_or(ModerationError::GuildUnavailable)?;
        let channel = guild
            .channels
            .get(&command.channel_id)
            .cloned()
            .ok_or(ModerationError::MissingChannel)?;
        let member = command
            .member
            .as_deref()
            .ok_or(ModerationError::MissingMember)?;
        let permissions = match parsed.permission().scope() {
            PermissionScope::Guild => guild.member_permissions(member),
            PermissionScope::Channel => guild.user_permissions_in(&channel, member),
        };

        (channel, permissions)
    };

    let backend = DiscordBackend {
        http: &ctx.http,
        guild_id,
    };

    run(&backend, &parsed, &channel, permissions).await
}

async fn run<B: ModerationBackend>(
    backend: &B,
    command: &ModerationCommand,
    channel: &GuildChannel,
    member_permissions: Permissions,
) -> Result<String, ModerationError> {
    command.permission().authorize(member_permissions)?;
    execute(backend, command, channel).await
}

async fn execute<B: ModerationBackend>(
    backend: &B,
    command: &ModerationCommand,
    channel: &GuildChannel,
) -> Result<String, ModerationError> {
    match command {
        ModerationCommand::Ban {
            user,
            reason,
            delete_message_days,
        } => {
            backend
                .ban(*user, reason.as_deref(), *delete_message_days)
                .await?;

            Ok(format!("Banned {}.", user.mention()))
        }
        ModerationCommand::Kick { user, reason } => {
            backend.kick(*user, reason.as_deref()).await?;
            Ok(format!("Kicked {}.", user.mention()))
        }
        ModerationCommand::Purge { count } => {
            let deleted = backend.purge(channel, *count).await?;

            Ok(match deleted {
                0 => "No messages to delete.".to_owned(),
                1 => "Deleted 1 message.".to_owned(),
                deleted => format!("Deleted {deleted} messages."),
            })
        }
        ModerationCommand::Lock => {
            backend.lock(channel).await?;
            Ok(format!("Locked {}.", channel.id.mention()))
        }
        ModerationCommand::Unlock => {
            backend.unlock(channel).await?;
            Ok(format!("Unlocked {}.", channel.id.mention()))
        }
    }
}

trait ModerationBackend {
    async fn ban(
        &self,
        user: UserId,
        reason: Option<&str>,
        delete_message_days: u8,
    ) -> Result<(), ModerationError>;
    async fn kick(&self, user: UserId, reason: Option<&str>) -> Result<(), ModerationError>;
    async fn purge(&self, channel: &GuildChannel, count: u8) -> Result<u64, ModerationError>;
    async fn lock(&self, channel: &GuildChannel) -> Result<(), ModerationError>;
    async fn unlock(&self, channel: &GuildChannel) -> Result<(), ModerationError>;
}

struct DiscordBackend<'a> {
    http: &'a Http,
    guild_id: GuildId,
}

impl ModerationBackend for DiscordBackend<'_> {
    async fn ban(
        &self,
        user: UserId,
        reason: Option<&str>,
        delete_message_days: u8,
    ) -> Result<(), ModerationError> {
        self.http
            .ban_user(self.guild_id, user, delete_message_days, reason)
            .await?;

        Ok(())
    }

    async fn kick(&self, user: UserId, reason: Option<&str>) -> Result<(), ModerationError> {
        self.http.kick_member(self.guild_id, user, reason).await?;
        Ok(())
    }

    async fn purge(&self, channel: &GuildChannel, count: u8) -> Result<u64, ModerationError> {
        let messages = self
            .http
            .get_messages(channel.id, None, Some(count))
            .await?;
        let message_ids: Vec<MessageId> = messages.iter().map(|message| message.id).collect();

        if message_ids.is_empty() {
            return Ok(0);
        }

        channel.id.delete_messages(self.http, message_ids).await?;

        Ok(u64::try_from(messages.len()).unwrap_or(u64::MAX))
    }

    async fn lock(&self, channel: &GuildChannel) -> Result<(), ModerationError> {
        set_locked(self.http, self.guild_id, channel, true).await
    }

    async fn unlock(&self, channel: &GuildChannel) -> Result<(), ModerationError> {
        set_locked(self.http, self.guild_id, channel, false).await
    }
}

async fn set_locked(
    http: &Http,
    guild_id: GuildId,
    channel: &GuildChannel,
    locked: bool,
) -> Result<(), ModerationError> {
    let everyone = guild_id.everyone_role();
    let existing = channel
        .permission_overwrites
        .iter()
        .find(|overwrite| overwrite.kind == PermissionOverwriteType::Role(everyone))
        .cloned();
    let Some(overwrite) = locked_overwrite(existing, guild_id, locked) else {
        return Ok(());
    };

    channel.id.create_permission(http, overwrite).await?;

    Ok(())
}

fn locked_overwrite(
    existing: Option<PermissionOverwrite>,
    guild_id: GuildId,
    locked: bool,
) -> Option<PermissionOverwrite> {
    if !locked && existing.is_none() {
        return None;
    }

    let everyone = guild_id.everyone_role();
    let mut overwrite = existing.unwrap_or(PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Role(everyone),
    });
    let send_messages = Permissions::SEND_MESSAGES;

    if locked {
        overwrite.allow.remove(send_messages);
        overwrite.deny.insert(send_messages);
    } else {
        overwrite.deny.remove(send_messages);
    }

    Some(overwrite)
}

async fn respond_ephemeral(
    ctx: &Context,
    command: &CommandInteraction,
    content: impl Into<String>,
) -> Result<(), BotError> {
    command
        .create_response(&ctx.http, ephemeral_response(content))
        .await?;

    Ok(())
}

fn ephemeral_response(content: impl Into<String>) -> CreateInteractionResponse {
    CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    )
}

#[derive(Debug, Error)]
enum ModerationError {
    #[error("moderation commands only work in a server")]
    MissingGuild,
    #[error("the server data is unavailable; try again")]
    GuildUnavailable,
    #[error("your server member data is unavailable; try again")]
    MissingMember,
    #[error("the current channel is unavailable")]
    MissingChannel,
    #[error("invalid moderation command")]
    InvalidCommand,
    #[error(transparent)]
    PermissionDenied(#[from] PermissionDenied),
    #[error("moderation action failed: {0}")]
    Discord(#[source] Box<serenity::Error>),
}

impl From<serenity::Error> for ModerationError {
    fn from(error: serenity::Error) -> Self {
        Self::Discord(Box::new(error))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use serde_json::json;
    use serenity::model::application::CommandData;
    use serenity::model::channel::{
        GuildChannel, MessageFlags, PermissionOverwrite, PermissionOverwriteType,
    };
    use serenity::model::id::{ChannelId, GuildId, UserId};
    use serenity::model::permissions::Permissions;

    use super::{
        ModerationBackend, ModerationCommand, ModerationError, ephemeral_response,
        locked_overwrite, run,
    };
    use crate::commands::permissions::PermissionScope;

    fn data(options: serde_json::Value) -> Result<CommandData, serde_json::Error> {
        serde_json::from_value(json!({
            "id": "1",
            "name": "mod",
            "type": 1,
            "options": options,
        }))
    }

    fn ban() -> ModerationCommand {
        ModerationCommand::Ban {
            user: UserId::new(42),
            reason: Some("spam".to_owned()),
            delete_message_days: 2,
        }
    }

    fn kick() -> ModerationCommand {
        ModerationCommand::Kick {
            user: UserId::new(42),
            reason: None,
        }
    }

    fn purge() -> ModerationCommand {
        ModerationCommand::Purge { count: 25 }
    }

    fn channel() -> GuildChannel {
        let mut channel = GuildChannel::default();
        channel.id = ChannelId::new(7);
        channel
    }

    #[test]
    fn parses_ban_options() -> Result<(), serde_json::Error> {
        let data = data(json!([{
            "name": "ban",
            "type": 1,
            "options": [
                { "name": "user", "type": 6, "value": "42" },
                { "name": "reason", "type": 3, "value": "  spam  " },
                { "name": "delete_message_days", "type": 4, "value": 2 }
            ]
        }]))?;

        assert_eq!(ModerationCommand::parse(&data), Some(ban()));
        Ok(())
    }

    #[test]
    fn parses_kick_purge_lock_and_unlock() -> Result<(), serde_json::Error> {
        let kick_data = data(json!([{
            "name": "kick",
            "type": 1,
            "options": [{ "name": "user", "type": 6, "value": "42" }]
        }]))?;
        let purge_data = data(json!([{
            "name": "purge",
            "type": 1,
            "options": [{ "name": "count", "type": 4, "value": 25 }]
        }]))?;
        let lock_data = data(json!([{ "name": "lock", "type": 1, "options": [] }]))?;
        let unlock_data = data(json!([{ "name": "unlock", "type": 1, "options": [] }]))?;

        assert_eq!(ModerationCommand::parse(&kick_data), Some(kick()));
        assert_eq!(ModerationCommand::parse(&purge_data), Some(purge()));
        assert_eq!(
            ModerationCommand::parse(&lock_data),
            Some(ModerationCommand::Lock)
        );
        assert_eq!(
            ModerationCommand::parse(&unlock_data),
            Some(ModerationCommand::Unlock)
        );
        Ok(())
    }

    #[test]
    fn commands_declare_the_expected_permission_scopes() {
        assert_eq!(ban().permission().scope(), PermissionScope::Guild);
        assert_eq!(kick().permission().scope(), PermissionScope::Guild);
        assert_eq!(purge().permission().scope(), PermissionScope::Channel);
        assert_eq!(
            ModerationCommand::Lock.permission().scope(),
            PermissionScope::Channel
        );
        assert_eq!(
            ModerationCommand::Unlock.permission().scope(),
            PermissionScope::Channel
        );
    }

    #[test]
    fn rejects_missing_or_out_of_range_options() -> Result<(), serde_json::Error> {
        let missing_user = data(json!([{
            "name": "kick",
            "type": 1,
            "options": []
        }]))?;
        let zero_count = data(json!([{
            "name": "purge",
            "type": 1,
            "options": [{ "name": "count", "type": 4, "value": 0 }]
        }]))?;
        let too_many = data(json!([{
            "name": "purge",
            "type": 1,
            "options": [{ "name": "count", "type": 4, "value": 101 }]
        }]))?;

        assert_eq!(ModerationCommand::parse(&missing_user), None);
        assert_eq!(ModerationCommand::parse(&zero_count), None);
        assert_eq!(ModerationCommand::parse(&too_many), None);
        Ok(())
    }

    #[test]
    fn lock_preserves_other_overwrite_bits() {
        let guild_id = GuildId::new(1);
        let existing = PermissionOverwrite {
            allow: Permissions::ADD_REACTIONS | Permissions::SEND_MESSAGES,
            deny: Permissions::ATTACH_FILES,
            kind: PermissionOverwriteType::Role(guild_id.everyone_role()),
        };

        let overwrite = locked_overwrite(Some(existing), guild_id, true).expect("locked overwrite");

        assert!(overwrite.allow.contains(Permissions::ADD_REACTIONS));
        assert!(!overwrite.allow.contains(Permissions::SEND_MESSAGES));
        assert!(overwrite.deny.contains(Permissions::ATTACH_FILES));
        assert!(overwrite.deny.contains(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn unlock_removes_only_the_send_deny() {
        let guild_id = GuildId::new(1);
        let existing = PermissionOverwrite {
            allow: Permissions::ADD_REACTIONS,
            deny: Permissions::ATTACH_FILES | Permissions::SEND_MESSAGES,
            kind: PermissionOverwriteType::Role(guild_id.everyone_role()),
        };

        let overwrite =
            locked_overwrite(Some(existing), guild_id, false).expect("unlocked overwrite");

        assert!(overwrite.allow.contains(Permissions::ADD_REACTIONS));
        assert!(overwrite.deny.contains(Permissions::ATTACH_FILES));
        assert!(!overwrite.deny.contains(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn unlock_without_an_overwrite_skips_the_update() {
        assert!(locked_overwrite(None, GuildId::new(1), false).is_none());
    }

    #[test]
    fn command_responses_are_ephemeral() -> Result<(), serde_json::Error> {
        let value = serde_json::to_value(ephemeral_response("ok"))?;

        assert_eq!(
            value["data"]["flags"].as_u64(),
            Some(MessageFlags::EPHEMERAL.bits())
        );
        Ok(())
    }

    #[tokio::test]
    async fn permission_is_checked_before_every_action() {
        let backend = FakeBackend::default();
        let channel = channel();

        for command in [
            ban(),
            kick(),
            purge(),
            ModerationCommand::Lock,
            ModerationCommand::Unlock,
        ] {
            let error = run(&backend, &command, &channel, Permissions::empty())
                .await
                .expect_err("permission is enforced");

            assert!(matches!(error, ModerationError::PermissionDenied(_)));
        }

        assert!(backend.calls.borrow().is_empty());
    }

    #[tokio::test]
    async fn ban_runs_for_an_authorized_moderator() -> Result<(), ModerationError> {
        let backend = FakeBackend::default();
        let message = run(&backend, &ban(), &channel(), Permissions::BAN_MEMBERS).await?;

        assert_eq!(message, "Banned <@42>.");
        assert_eq!(
            backend.calls.borrow().as_slice(),
            [Call::Ban {
                user: UserId::new(42),
                reason: Some("spam".to_owned()),
                delete_message_days: 2,
            }]
        );
        Ok(())
    }

    #[tokio::test]
    async fn kick_runs_for_an_authorized_moderator() -> Result<(), ModerationError> {
        let backend = FakeBackend::default();
        let message = run(&backend, &kick(), &channel(), Permissions::KICK_MEMBERS).await?;

        assert_eq!(message, "Kicked <@42>.");
        assert_eq!(
            backend.calls.borrow().as_slice(),
            [Call::Kick {
                user: UserId::new(42),
                reason: None,
            }]
        );
        Ok(())
    }

    #[tokio::test]
    async fn purge_runs_and_reports_the_deleted_count() -> Result<(), ModerationError> {
        let backend = FakeBackend {
            purge_count: 3,
            ..FakeBackend::default()
        };
        let message = run(&backend, &purge(), &channel(), Permissions::MANAGE_MESSAGES).await?;

        assert_eq!(message, "Deleted 3 messages.");
        assert_eq!(
            backend.calls.borrow().as_slice(),
            [Call::Purge { count: 25 }]
        );
        Ok(())
    }

    #[tokio::test]
    async fn lock_and_unlock_run_for_an_authorized_moderator() -> Result<(), ModerationError> {
        let backend = FakeBackend::default();
        let channel = channel();

        let locked = run(
            &backend,
            &ModerationCommand::Lock,
            &channel,
            Permissions::MANAGE_CHANNELS,
        )
        .await?;
        let unlocked = run(
            &backend,
            &ModerationCommand::Unlock,
            &channel,
            Permissions::MANAGE_CHANNELS,
        )
        .await?;

        assert_eq!(locked, "Locked <#7>.");
        assert_eq!(unlocked, "Unlocked <#7>.");
        assert_eq!(
            backend.calls.borrow().as_slice(),
            [Call::Lock, Call::Unlock]
        );
        Ok(())
    }

    #[derive(Default)]
    struct FakeBackend {
        calls: RefCell<Vec<Call>>,
        purge_count: u64,
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Call {
        Ban {
            user: UserId,
            reason: Option<String>,
            delete_message_days: u8,
        },
        Kick {
            user: UserId,
            reason: Option<String>,
        },
        Purge {
            count: u8,
        },
        Lock,
        Unlock,
    }

    impl ModerationBackend for FakeBackend {
        async fn ban(
            &self,
            user: UserId,
            reason: Option<&str>,
            delete_message_days: u8,
        ) -> Result<(), ModerationError> {
            self.calls.borrow_mut().push(Call::Ban {
                user,
                reason: reason.map(str::to_owned),
                delete_message_days,
            });
            Ok(())
        }

        async fn kick(&self, user: UserId, reason: Option<&str>) -> Result<(), ModerationError> {
            self.calls.borrow_mut().push(Call::Kick {
                user,
                reason: reason.map(str::to_owned),
            });
            Ok(())
        }

        async fn purge(&self, _channel: &GuildChannel, count: u8) -> Result<u64, ModerationError> {
            self.calls.borrow_mut().push(Call::Purge { count });
            Ok(self.purge_count)
        }

        async fn lock(&self, _channel: &GuildChannel) -> Result<(), ModerationError> {
            self.calls.borrow_mut().push(Call::Lock);
            Ok(())
        }

        async fn unlock(&self, _channel: &GuildChannel) -> Result<(), ModerationError> {
            self.calls.borrow_mut().push(Call::Unlock);
            Ok(())
        }
    }
}
