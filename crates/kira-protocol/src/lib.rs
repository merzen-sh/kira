//! Request and response types shared by the Kira server and its clients.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Health information returned by `GET /health`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct HealthResponse {
    /// Service status, `ok` when the server is healthy.
    pub status: String,
    /// Version of the running `kira` binary.
    pub version: String,
}

impl HealthResponse {
    /// Build the healthy response for this binary's version.
    #[must_use]
    pub fn ok() -> Self {
        Self {
            status: "ok".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

/// Details of the Discord server targeted by `GET /guild`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct GuildDetails {
    /// Discord server ID.
    pub id: String,
    /// Server display name.
    pub name: String,
    /// Server description, when set.
    pub description: Option<String>,
    /// URL of the server icon, when set.
    pub icon_url: Option<String>,
    /// ID of the server owner.
    pub owner_id: String,
    /// Total number of members in the server.
    pub member_count: u64,
    /// Approximate number of online members, when known.
    pub presence_count: Option<u64>,
    /// Preferred locale of the server.
    pub preferred_locale: String,
    /// Enabled guild features.
    pub features: Vec<String>,
}

impl GuildDetails {
    /// Build details with only the required fields populated.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            icon_url: None,
            owner_id: String::new(),
            member_count: 0,
            presence_count: None,
            preferred_locale: String::new(),
            features: Vec::new(),
        }
    }
}

/// A channel belonging to the Discord server targeted by `GET /guild/channels`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct ChannelDetails {
    /// Discord channel ID.
    pub id: String,
    /// ID of the server the channel belongs to.
    pub guild_id: String,
    /// Channel name.
    pub name: String,
    /// Channel type, such as `text`, `voice`, or `category`.
    pub kind: String,
    /// Sort position within the server.
    pub position: u16,
    /// ID of the parent category or channel, when set.
    pub parent_id: Option<String>,
}

impl ChannelDetails {
    /// Build details with only the required fields populated.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        guild_id: impl Into<String>,
        name: impl Into<String>,
        kind: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            guild_id: guild_id.into(),
            name: name.into(),
            kind: kind.into(),
            position: 0,
            parent_id: None,
        }
    }
}

/// A message posted to a Discord channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct SentMessage {
    /// Discord message ID.
    pub id: String,
    /// Channel that received the message.
    pub channel_id: String,
}

impl SentMessage {
    /// Build a sent message reference.
    #[must_use]
    pub fn new(id: impl Into<String>, channel_id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            channel_id: channel_id.into(),
        }
    }
}

/// A bug report submitted through the Discord `/report` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct Report {
    /// Report ID.
    pub id: i64,
    /// Discord user ID of the reporter.
    pub user_id: String,
    /// Short description of what is broken.
    pub title: String,
    /// Steps the reporter took to reproduce the problem.
    pub steps: String,
    /// What happened and what the reporter expected instead.
    pub details: String,
    /// Whether the report has been marked resolved.
    pub resolved: bool,
    /// Creation time as an ISO 8601 timestamp.
    pub created_at: String,
}

impl Report {
    /// Build a report with every field populated.
    #[must_use]
    pub fn new(
        id: i64,
        user_id: impl Into<String>,
        title: impl Into<String>,
        steps: impl Into<String>,
        details: impl Into<String>,
        resolved: bool,
        created_at: impl Into<String>,
    ) -> Self {
        Self {
            id,
            user_id: user_id.into(),
            title: title.into(),
            steps: steps.into(),
            details: details.into(),
            resolved,
            created_at: created_at.into(),
        }
    }
}

/// A page of bug reports returned by `GET /reports`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct ReportList {
    /// Reports on this page, newest first.
    pub reports: Vec<Report>,
    /// Total number of reports matching the search.
    pub total: u64,
    /// Page number, starting at 1.
    pub page: u32,
    /// Maximum number of reports per page.
    pub page_limit: u32,
}

impl ReportList {
    /// Build a page of reports.
    #[must_use]
    pub fn new(reports: Vec<Report>, total: u64, page: u32, page_limit: u32) -> Self {
        Self {
            reports,
            total,
            page,
            page_limit,
        }
    }
}

/// Resolution change submitted to `PATCH /reports/{id}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpdateReport {
    /// New resolved state.
    pub resolved: bool,
}

/// Credentials submitted to `POST /auth/login`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct LoginRequest {
    /// Dashboard PIN or password.
    pub pin: String,
}

/// Error payload returned when a request fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct ErrorResponse {
    /// Human-readable failure message.
    pub error: String,
}

impl ErrorResponse {
    /// Build an error payload from a human-readable message.
    #[must_use]
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_ok_serializes_expected_fields() -> Result<(), Box<dyn std::error::Error>> {
        let v = serde_json::to_value(HealthResponse::ok())?;
        assert_eq!(v["status"], "ok");
        assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
        Ok(())
    }

    #[test]
    fn error_response_serializes() -> Result<(), Box<dyn std::error::Error>> {
        let v = serde_json::to_value(ErrorResponse::new("boom"))?;
        assert_eq!(v["error"], "boom");
        Ok(())
    }

    #[test]
    fn channel_details_serializes_all_fields() -> Result<(), Box<dyn std::error::Error>> {
        let mut channel = ChannelDetails::new("10", "1", "general", "text");
        channel.position = 2;
        channel.parent_id = Some("9".into());

        let value = serde_json::to_value(channel)?;

        assert_eq!(value["id"], "10");
        assert_eq!(value["guild_id"], "1");
        assert_eq!(value["name"], "general");
        assert_eq!(value["kind"], "text");
        assert_eq!(value["position"], 2);
        assert_eq!(value["parent_id"], "9");
        Ok(())
    }

    #[test]
    fn guild_details_serializes_all_fields() -> Result<(), Box<dyn std::error::Error>> {
        let mut guild = GuildDetails::new("1", "Alpha");
        guild.description = Some("A test server".into());
        guild.icon_url = Some("https://cdn.example/icon.webp".into());
        guild.owner_id = "42".into();
        guild.member_count = 12;
        guild.presence_count = Some(3);
        guild.preferred_locale = "en-US".into();
        guild.features = vec!["COMMUNITY".into()];

        let value = serde_json::to_value(guild)?;

        assert_eq!(value["id"], "1");
        assert_eq!(value["name"], "Alpha");
        assert_eq!(value["description"], "A test server");
        assert_eq!(value["icon_url"], "https://cdn.example/icon.webp");
        assert_eq!(value["owner_id"], "42");
        assert_eq!(value["member_count"], 12);
        assert_eq!(value["presence_count"], 3);
        assert_eq!(value["preferred_locale"], "en-US");
        assert_eq!(value["features"][0], "COMMUNITY");
        Ok(())
    }
}
