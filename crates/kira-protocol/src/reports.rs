//! Bug report request and response types.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A bug report submitted through the Discord `/report` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct Report {
    /// Report ID.
    pub id: i64,
    /// Discord user ID of the reporter.
    pub user_id: String,
    /// Reporter username resolved from the tracked guild user, when available.
    pub user_name: Option<String>,
    /// Reporter display name resolved from the tracked guild user, when available.
    pub display_name: Option<String>,
    /// Reporter avatar URL resolved from the tracked guild user, when available.
    pub avatar_url: Option<String>,
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
    #[expect(
        clippy::too_many_arguments,
        reason = "a report row maps one-to-one onto these fields"
    )]
    pub fn new(
        id: i64,
        user_id: impl Into<String>,
        user_name: Option<String>,
        display_name: Option<String>,
        avatar_url: Option<String>,
        title: impl Into<String>,
        steps: impl Into<String>,
        details: impl Into<String>,
        resolved: bool,
        created_at: impl Into<String>,
    ) -> Self {
        Self {
            id,
            user_id: user_id.into(),
            user_name,
            display_name,
            avatar_url,
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
    /// Total number of reports matching the filters.
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

/// Aggregate counts for bug reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct ReportStats {
    /// Total number of reports.
    pub total: u64,
    /// Number of reports marked resolved.
    pub resolved: u64,
    /// Number of reports still open.
    pub open: u64,
}

impl ReportStats {
    /// Build stats from the total and resolved counts.
    #[must_use]
    pub fn new(total: u64, resolved: u64) -> Self {
        Self {
            total,
            resolved,
            open: total.saturating_sub(resolved),
        }
    }
}

/// Resolution change submitted to `PATCH /reports/{id}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct UpdateReport {
    /// New resolved state.
    pub resolved: bool,
}

#[cfg(test)]
mod tests {
    use super::UpdateReport;

    #[test]
    fn unknown_update_fields_are_rejected() {
        assert!(serde_json::from_str::<UpdateReport>(r#"{"resolved":true,"extra":1}"#).is_err());
    }
}
