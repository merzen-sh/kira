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
