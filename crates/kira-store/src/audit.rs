//! Generic audit event persistence.

use kira_protocol::{AuditEntity, AuditEvent, AuditEventList, AuditFacets};
use serde_json::{Map, Value};
use sqlx::FromRow;

use crate::query::{escape_like, page_offset};
use crate::{Database, StoreError};

const EVENT_COLUMNS: &str = "id, action, source, actor_kind, actor_id, actor_name, \
    target_kind, target_id, target_name, metadata, occurred_at, recorded_at";

const FILTER: &str = "(? = '' \
        OR action LIKE ? ESCAPE '\\' \
        OR actor_kind LIKE ? ESCAPE '\\' \
        OR COALESCE(actor_id, '') LIKE ? ESCAPE '\\' \
        OR COALESCE(actor_name, '') LIKE ? ESCAPE '\\' \
        OR COALESCE(target_kind, '') LIKE ? ESCAPE '\\' \
        OR COALESCE(target_id, '') LIKE ? ESCAPE '\\' \
        OR COALESCE(target_name, '') LIKE ? ESCAPE '\\') \
    AND (? IS NULL OR action = ?) \
    AND (? IS NULL OR actor_kind = ?) \
    AND (? IS NULL OR source = ?)";

/// An audit event to insert into storage.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct NewAuditEvent<'a> {
    /// Action that occurred.
    pub action: &'a str,
    /// Where the event originated.
    pub source: &'a str,
    /// Who or what performed the action.
    pub actor: &'a AuditEntity,
    /// What the action affected, when it has a target.
    pub target: Option<&'a AuditEntity>,
    /// Structured JSON object with event-specific details.
    pub metadata: &'a Map<String, Value>,
}

impl<'a> NewAuditEvent<'a> {
    /// Build an audit event from its fields.
    #[must_use]
    pub const fn new(
        action: &'a str,
        source: &'a str,
        actor: &'a AuditEntity,
        target: Option<&'a AuditEntity>,
        metadata: &'a Map<String, Value>,
    ) -> Self {
        Self {
            action,
            source,
            actor,
            target,
            metadata,
        }
    }
}

/// Queries for audit events.
#[derive(Clone)]
pub struct AuditRepository {
    db: Database,
}

impl AuditRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    /// Insert an event and return it with its generated ID and timestamps.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the insert fails.
    pub async fn record(&self, event: &NewAuditEvent<'_>) -> Result<AuditEvent, StoreError> {
        let sql = format!(
            "INSERT INTO audit_events \
                (action, source, actor_kind, actor_id, actor_name, target_kind, target_id, \
                 target_name, metadata) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
             RETURNING {EVENT_COLUMNS}"
        );
        let (target_kind, target_id, target_name) =
            event.target.map_or((None, None, None), |target| {
                (
                    Some(target.kind.as_str()),
                    target.id.as_deref(),
                    target.name.as_deref(),
                )
            });

        let row = sqlx::query_as::<_, AuditEventRow>(&sql)
            .bind(event.action)
            .bind(event.source)
            .bind(&event.actor.kind)
            .bind(&event.actor.id)
            .bind(&event.actor.name)
            .bind(target_kind)
            .bind(target_id)
            .bind(target_name)
            .bind(Value::Object(event.metadata.clone()).to_string())
            .fetch_one(self.db.pool())
            .await?;

        Ok(row.into())
    }

    /// List audit events newest first, optionally filtered by a
    /// case-insensitive substring, exact action, actor kind, or source.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when either query fails.
    pub async fn list_events(
        &self,
        page: u32,
        page_limit: u32,
        search: Option<&str>,
        action: Option<&str>,
        actor_kind: Option<&str>,
        source: Option<&str>,
    ) -> Result<AuditEventList, StoreError> {
        let search = search.map(str::trim).filter(|value| !value.is_empty());
        let search = search.unwrap_or_default();
        let pattern = format!("%{}%", escape_like(search));
        let (limit, offset) = page_offset(page, page_limit);

        let count_sql = format!("SELECT COUNT(*) FROM audit_events WHERE {FILTER}");
        let total: i64 = sqlx::query_scalar(&count_sql)
            .bind(search)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(action)
            .bind(action)
            .bind(actor_kind)
            .bind(actor_kind)
            .bind(source)
            .bind(source)
            .fetch_one(self.db.pool())
            .await?;

        let list_sql = format!(
            "SELECT {EVENT_COLUMNS} FROM audit_events WHERE {FILTER} \
             ORDER BY occurred_at DESC, id DESC LIMIT ? OFFSET ?"
        );
        let rows = sqlx::query_as::<_, AuditEventRow>(&list_sql)
            .bind(search)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(action)
            .bind(action)
            .bind(actor_kind)
            .bind(actor_kind)
            .bind(source)
            .bind(source)
            .bind(limit)
            .bind(offset)
            .fetch_all(self.db.pool())
            .await?;

        Ok(AuditEventList::new(
            rows.into_iter().map(AuditEvent::from).collect(),
            u64::try_from(total).unwrap_or_default(),
            page,
            page_limit,
        ))
    }

    /// Distinct action and actor kind values, sorted alphabetically.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when any facet query fails.
    pub async fn facets(&self) -> Result<AuditFacets, StoreError> {
        let actions: Vec<String> =
            sqlx::query_scalar("SELECT DISTINCT action FROM audit_events ORDER BY action")
                .fetch_all(self.db.pool())
                .await?;
        let actor_kinds: Vec<String> =
            sqlx::query_scalar("SELECT DISTINCT actor_kind FROM audit_events ORDER BY actor_kind")
                .fetch_all(self.db.pool())
                .await?;
        let sources: Vec<String> =
            sqlx::query_scalar("SELECT DISTINCT source FROM audit_events ORDER BY source")
                .fetch_all(self.db.pool())
                .await?;

        Ok(AuditFacets::new(actions, actor_kinds, sources))
    }
}

#[derive(Debug, FromRow)]
struct AuditEventRow {
    id: i64,
    action: String,
    source: String,
    actor_kind: String,
    actor_id: Option<String>,
    actor_name: Option<String>,
    target_kind: Option<String>,
    target_id: Option<String>,
    target_name: Option<String>,
    metadata: String,
    occurred_at: String,
    recorded_at: String,
}

impl From<AuditEventRow> for AuditEvent {
    fn from(row: AuditEventRow) -> Self {
        let target = row
            .target_kind
            .map(|kind| AuditEntity::new(kind, row.target_id, row.target_name));
        let metadata = match serde_json::from_str(&row.metadata) {
            Ok(Value::Object(metadata)) => metadata,
            _ => {
                tracing::warn!(
                    event_id = row.id,
                    "audit event metadata is not a JSON object; returning an empty map"
                );
                Map::new()
            }
        };

        Self::new(
            row.id,
            row.action,
            row.source,
            AuditEntity::new(row.actor_kind, row.actor_id, row.actor_name),
            target,
            metadata,
            row.occurred_at,
            row.recorded_at,
        )
    }
}

#[cfg(test)]
mod tests {
    use kira_protocol::{AuditEntity, SOURCE_BOT, SOURCE_DASHBOARD, SOURCE_SYSTEM};
    use serde_json::json;

    use super::NewAuditEvent;
    use crate::StoreError;
    use crate::memory;

    fn actor(kind: &str, id: &str, name: &str) -> AuditEntity {
        AuditEntity::new(kind, Some(id.to_owned()), Some(name.to_owned()))
    }

    fn metadata(value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        match value {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        }
    }

    #[tokio::test]
    async fn events_are_recorded_with_defaults_and_listed_newest_first() -> Result<(), StoreError> {
        let audit = memory().await.audit();
        let alice = actor("user", "42", "Alice");
        let report_metadata = metadata(json!({ "report_id": 7 }));
        let empty = metadata(json!({}));

        let first = audit
            .record(&NewAuditEvent::new(
                "report.created",
                SOURCE_BOT,
                &alice,
                None,
                &report_metadata,
            ))
            .await?;
        let second = audit
            .record(&NewAuditEvent::new(
                "report.resolved",
                SOURCE_DASHBOARD,
                &alice,
                None,
                &empty,
            ))
            .await?;

        assert!(!first.occurred_at.is_empty());
        assert!(!first.recorded_at.is_empty());
        assert_eq!(first.metadata, report_metadata);
        assert!(second.metadata.is_empty());
        assert!(second.id > first.id);

        let page = audit.list_events(1, 20, None, None, None, None).await?;
        assert_eq!(page.total, 2);
        assert_eq!(page.events[0].action, "report.resolved");
        assert_eq!(page.events[1].action, "report.created");
        Ok(())
    }

    #[tokio::test]
    async fn search_action_and_actor_filters_narrow_the_list() -> Result<(), StoreError> {
        let audit = memory().await.audit();
        let alice = actor("user", "42", "Alice");
        let system = actor("system", "scheduler", "Scheduler");
        let percent = actor("user", "7", "Percent % Human");
        let trace = metadata(json!({ "trace": "abc123" }));
        let empty = metadata(json!({}));

        audit
            .record(&NewAuditEvent::new(
                "report.created",
                SOURCE_BOT,
                &alice,
                None,
                &trace,
            ))
            .await?;
        audit
            .record(&NewAuditEvent::new(
                "report.resolved",
                SOURCE_DASHBOARD,
                &alice,
                None,
                &empty,
            ))
            .await?;
        audit
            .record(&NewAuditEvent::new(
                "system.startup",
                SOURCE_SYSTEM,
                &system,
                None,
                &empty,
            ))
            .await?;
        audit
            .record(&NewAuditEvent::new(
                "report.created",
                SOURCE_DASHBOARD,
                &percent,
                None,
                &empty,
            ))
            .await?;

        let by_actor = audit
            .list_events(1, 20, Some("alice"), None, None, None)
            .await?;
        let by_action = audit
            .list_events(1, 20, Some("startup"), None, None, None)
            .await?;
        let by_metadata = audit
            .list_events(1, 20, Some("abc123"), None, None, None)
            .await?;
        let exact_action = audit
            .list_events(1, 20, None, Some("report.resolved"), None, None)
            .await?;
        let actor_kind = audit
            .list_events(1, 20, None, None, Some("system"), None)
            .await?;
        let source = audit
            .list_events(1, 20, None, None, None, Some(SOURCE_BOT))
            .await?;
        let missing = audit
            .list_events(1, 20, Some("absent"), None, None, None)
            .await?;
        let literal_percent = audit
            .list_events(1, 20, Some("%"), None, None, None)
            .await?;
        let underscore = audit
            .list_events(1, 20, Some("_"), None, None, None)
            .await?;

        assert_eq!(by_actor.total, 2);
        assert_eq!(by_action.total, 1);
        assert_eq!(by_metadata.total, 0);
        assert_eq!(exact_action.total, 1);
        assert_eq!(actor_kind.total, 1);
        assert_eq!(actor_kind.events[0].action, "system.startup");
        assert_eq!(source.total, 1);
        assert_eq!(source.events[0].action, "report.created");
        assert_eq!(missing.total, 0);
        assert_eq!(literal_percent.total, 1);
        assert_eq!(underscore.total, 0);
        Ok(())
    }

    #[tokio::test]
    async fn pagination_returns_the_requested_slice() -> Result<(), StoreError> {
        let audit = memory().await.audit();
        let alice = actor("user", "42", "Alice");

        for index in 0..3 {
            let event_metadata = metadata(json!({ "index": index }));
            audit
                .record(&NewAuditEvent::new(
                    "report.created",
                    SOURCE_BOT,
                    &alice,
                    None,
                    &event_metadata,
                ))
                .await?;
        }

        let first = audit.list_events(1, 2, None, None, None, None).await?;
        let second = audit.list_events(2, 2, None, None, None, None).await?;

        assert_eq!(first.total, 3);
        assert_eq!(first.events.len(), 2);
        assert_eq!(first.events[0].metadata["index"], 2);
        assert_eq!(second.events.len(), 1);
        assert_eq!(second.events[0].metadata["index"], 0);
        Ok(())
    }

    #[tokio::test]
    async fn facets_list_distinct_sorted_values() -> Result<(), StoreError> {
        let audit = memory().await.audit();
        let alice = actor("user", "42", "Alice");
        let system = actor("system", "scheduler", "Scheduler");
        let empty = metadata(json!({}));

        audit
            .record(&NewAuditEvent::new(
                "report.resolved",
                SOURCE_DASHBOARD,
                &alice,
                None,
                &empty,
            ))
            .await?;
        audit
            .record(&NewAuditEvent::new(
                "report.created",
                SOURCE_BOT,
                &alice,
                None,
                &empty,
            ))
            .await?;
        audit
            .record(&NewAuditEvent::new(
                "system.startup",
                SOURCE_SYSTEM,
                &system,
                None,
                &empty,
            ))
            .await?;

        let facets = audit.facets().await?;
        assert_eq!(
            facets.actions,
            ["report.created", "report.resolved", "system.startup"]
        );
        assert_eq!(facets.actor_kinds, ["system", "user"]);
        assert_eq!(facets.sources, ["bot", "dashboard", "system"]);
        Ok(())
    }
}
