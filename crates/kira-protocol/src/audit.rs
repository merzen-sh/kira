//! Generic audit event request and response types.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Audit source for actions taken through the dashboard.
pub const SOURCE_DASHBOARD: &str = "dashboard";
/// Audit source for actions taken by the Discord bot.
pub const SOURCE_BOT: &str = "bot";
/// Audit source for actions taken by Kira itself.
pub const SOURCE_SYSTEM: &str = "system";

/// A party that caused or was affected by an audit event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct AuditEntity {
    /// Entity type, for example `user`, `service`, or `system`.
    pub kind: String,
    /// Stable identifier within the kind, when one exists.
    pub id: Option<String>,
    /// Human-readable label captured at event time.
    pub name: Option<String>,
}

impl AuditEntity {
    /// Build an audit entity with every field populated.
    #[must_use]
    pub fn new(kind: impl Into<String>, id: Option<String>, name: Option<String>) -> Self {
        Self {
            kind: kind.into(),
            id,
            name,
        }
    }
}

/// A stored audit event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct AuditEvent {
    /// Event ID.
    pub id: i64,
    /// Action that occurred, for example `report.resolved`.
    pub action: String,
    /// Where the event originated: `dashboard`, `bot`, or `system`.
    pub source: String,
    /// Who or what performed the action.
    pub actor: AuditEntity,
    /// What the action affected, when it has a target.
    pub target: Option<AuditEntity>,
    /// Structured JSON object with event-specific details.
    pub metadata: serde_json::Map<String, serde_json::Value>,
    /// Time the event happened, as an ISO 8601 timestamp.
    pub occurred_at: String,
    /// Time the event was stored, as an ISO 8601 timestamp.
    pub recorded_at: String,
}

impl AuditEvent {
    /// Build an audit event with every field populated.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "a full event row maps one-to-one onto these fields"
    )]
    pub fn new(
        id: i64,
        action: impl Into<String>,
        source: impl Into<String>,
        actor: AuditEntity,
        target: Option<AuditEntity>,
        metadata: serde_json::Map<String, serde_json::Value>,
        occurred_at: impl Into<String>,
        recorded_at: impl Into<String>,
    ) -> Self {
        Self {
            id,
            action: action.into(),
            source: source.into(),
            actor,
            target,
            metadata,
            occurred_at: occurred_at.into(),
            recorded_at: recorded_at.into(),
        }
    }
}

/// A page of audit events returned by `GET /audit/events`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct AuditEventList {
    /// Events on this page, newest first.
    pub events: Vec<AuditEvent>,
    /// Total number of events matching the filters.
    pub total: u64,
    /// Page number, starting at 1.
    pub page: u32,
    /// Maximum number of events per page.
    pub page_limit: u32,
}

impl AuditEventList {
    /// Build a page of audit events.
    #[must_use]
    pub fn new(events: Vec<AuditEvent>, total: u64, page: u32, page_limit: u32) -> Self {
        Self {
            events,
            total,
            page,
            page_limit,
        }
    }
}

/// Filter values available for audit event queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct AuditFacets {
    /// Distinct action values, sorted alphabetically.
    pub actions: Vec<String>,
    /// Distinct actor kinds, sorted alphabetically.
    pub actor_kinds: Vec<String>,
    /// Distinct event sources, sorted alphabetically.
    pub sources: Vec<String>,
}

impl AuditFacets {
    /// Build the available filter values.
    #[must_use]
    pub fn new(actions: Vec<String>, actor_kinds: Vec<String>, sources: Vec<String>) -> Self {
        Self {
            actions,
            actor_kinds,
            sources,
        }
    }
}
