//! Bot presence request and response types.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Discord online status of the bot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BotStatus {
    /// Visible and available.
    #[default]
    Online,
    /// Visible but marked idle.
    Idle,
    /// Visible but marked do not disturb.
    #[serde(rename = "dnd")]
    DoNotDisturb,
    /// Appears offline to other members.
    Invisible,
}

/// Kind of activity displayed in the bot's presence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActivityKind {
    /// Playing a game.
    #[default]
    Playing,
    /// Watching something.
    Watching,
    /// Listening to something.
    Listening,
    /// Competing in something.
    Competing,
    /// A custom status.
    Custom,
}

/// An activity displayed in the bot's presence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct BotActivity {
    /// Activity kind.
    pub kind: ActivityKind,
    /// Activity text.
    pub text: String,
}

/// Stored presence configuration for the bot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct BotPresence {
    /// Online status.
    pub status: BotStatus,
    /// Activity, when one is configured.
    pub activity: Option<BotActivity>,
}

impl BotActivity {
    /// Build an activity from its kind and text.
    #[must_use]
    pub fn new(kind: ActivityKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }
}

impl BotPresence {
    /// Build a presence from a status and optional activity.
    #[must_use]
    pub const fn new(status: BotStatus, activity: Option<BotActivity>) -> Self {
        Self { status, activity }
    }
}

impl BotStatus {
    /// Stable string used by the API, database, and audit metadata.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Idle => "idle",
            Self::DoNotDisturb => "dnd",
            Self::Invisible => "invisible",
        }
    }
}

impl ActivityKind {
    /// Stable string used by the API, database, and audit metadata.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Playing => "playing",
            Self::Watching => "watching",
            Self::Listening => "listening",
            Self::Competing => "competing",
            Self::Custom => "custom",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_serializes_defaults() -> Result<(), serde_json::Error> {
        let value = serde_json::to_value(BotPresence::default())?;
        assert_eq!(
            value,
            serde_json::json!({ "status": "online", "activity": null })
        );
        Ok(())
    }

    #[test]
    fn presence_round_trips_activity() -> Result<(), serde_json::Error> {
        let presence = BotPresence::new(
            BotStatus::DoNotDisturb,
            Some(BotActivity::new(ActivityKind::Watching, "Kira")),
        );

        let value = serde_json::to_value(&presence)?;
        assert_eq!(value["status"], "dnd");
        assert_eq!(value["activity"]["kind"], "watching");
        assert_eq!(value["activity"]["text"], "Kira");
        assert_eq!(serde_json::from_value::<BotPresence>(value)?, presence);
        Ok(())
    }

    #[test]
    fn unknown_status_is_rejected() {
        assert!(
            serde_json::from_value::<BotPresence>(
                serde_json::json!({ "status": "away", "activity": null })
            )
            .is_err()
        );
    }

    #[test]
    fn as_str_matches_serialized_values() -> Result<(), serde_json::Error> {
        assert_eq!(serde_json::to_value(BotStatus::DoNotDisturb)?, "dnd");
        assert_eq!(BotStatus::DoNotDisturb.as_str(), "dnd");
        assert_eq!(ActivityKind::Custom.as_str(), "custom");
        Ok(())
    }
}
