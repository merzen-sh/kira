//! Bug report persistence.

use kira_protocol::{Report, ReportList, ReportStats};
use sqlx::FromRow;

use crate::query::{escape_like, page_offset};
use crate::{Database, StoreError};

const REPORT_COLUMNS: &str = "r.id, r.user_id, u.user_name, u.display_name, u.avatar_url, \
    r.title, r.steps, r.details, r.resolved, r.created_at";

const REPORT_JOIN: &str = "reports r LEFT JOIN guild_users u ON u.user_id = r.user_id";

const REPORT_RETURNING: &str = "id, user_id, NULL AS user_name, NULL AS display_name, \
    NULL AS avatar_url, title, steps, details, resolved, created_at";

const FILTER: &str = "(? = '' \
        OR r.title LIKE ? ESCAPE '\\' \
        OR r.steps LIKE ? ESCAPE '\\' \
        OR r.details LIKE ? ESCAPE '\\' \
        OR r.user_id LIKE ? ESCAPE '\\' \
        OR COALESCE(u.user_name, '') LIKE ? ESCAPE '\\' \
        OR COALESCE(u.display_name, '') LIKE ? ESCAPE '\\') \
    AND (? IS NULL OR r.resolved = ?)";

/// A report to insert into storage.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct NewReport<'a> {
    /// Discord user ID of the reporter.
    pub user_id: &'a str,
    /// Short description of what is broken.
    pub title: &'a str,
    /// Steps the reporter took to reproduce the problem.
    pub steps: &'a str,
    /// What happened and what the reporter expected instead.
    pub details: &'a str,
}

impl<'a> NewReport<'a> {
    /// Build a report from its submitted fields.
    #[must_use]
    pub const fn new(user_id: &'a str, title: &'a str, steps: &'a str, details: &'a str) -> Self {
        Self {
            user_id,
            title,
            steps,
            details,
        }
    }
}

/// Result of changing a report's resolved state.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ResolveOutcome {
    /// Updated report.
    pub report: Report,
    /// Whether the stored state actually changed.
    pub changed: bool,
}

/// A report removed from storage.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DeletedReport {
    /// Deleted report ID.
    pub id: i64,
    /// Deleted report title.
    pub title: String,
}

/// Queries for bug reports.
#[derive(Clone)]
pub struct ReportRepository {
    db: Database,
}

impl ReportRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    /// Insert a report and return it with its generated ID, timestamp, and
    /// default unresolved state. Reporter fields are `None` on the returned
    /// value; read the report back to resolve them from tracked users.
    pub async fn create_report(&self, report: &NewReport<'_>) -> Result<Report, StoreError> {
        let sql = format!(
            "INSERT INTO reports (user_id, title, steps, details) \
             VALUES (?, ?, ?, ?) RETURNING {REPORT_RETURNING}"
        );

        let row = sqlx::query_as::<_, ReportRow>(&sql)
            .bind(report.user_id)
            .bind(report.title)
            .bind(report.steps)
            .bind(report.details)
            .fetch_one(self.db.pool())
            .await?;

        Ok(row.into())
    }

    /// List reports newest first, optionally filtered by a case-insensitive
    /// substring of the title, steps, details, or reporter, and by resolved state.
    pub async fn list_reports(
        &self,
        page: u32,
        page_limit: u32,
        search: Option<&str>,
        resolved: Option<bool>,
    ) -> Result<ReportList, StoreError> {
        let search = search.map(str::trim).filter(|value| !value.is_empty());
        let search = search.unwrap_or_default();
        let pattern = format!("%{}%", escape_like(search));
        let (limit, offset) = page_offset(page, page_limit);

        let count_sql = format!("SELECT COUNT(*) FROM {REPORT_JOIN} WHERE {FILTER}");
        let total: i64 = sqlx::query_scalar(&count_sql)
            .bind(search)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(resolved)
            .bind(resolved)
            .fetch_one(self.db.pool())
            .await?;

        let list_sql = format!(
            "SELECT {REPORT_COLUMNS} FROM {REPORT_JOIN} WHERE {FILTER} \
             ORDER BY r.id DESC LIMIT ? OFFSET ?"
        );
        let rows = sqlx::query_as::<_, ReportRow>(&list_sql)
            .bind(search)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(resolved)
            .bind(resolved)
            .bind(limit)
            .bind(offset)
            .fetch_all(self.db.pool())
            .await?;

        Ok(ReportList::new(
            rows.into_iter().map(Report::from).collect(),
            u64::try_from(total).unwrap_or_default(),
            page,
            page_limit,
        ))
    }

    /// Count every report and how many are resolved.
    pub async fn report_stats(&self) -> Result<ReportStats, StoreError> {
        let (total, resolved): (i64, i64) =
            sqlx::query_as("SELECT COUNT(*), COALESCE(SUM(resolved), 0) FROM reports")
                .fetch_one(self.db.pool())
                .await?;

        Ok(ReportStats::new(
            u64::try_from(total).unwrap_or_default(),
            u64::try_from(resolved).unwrap_or_default(),
        ))
    }

    /// Set the resolved state of a report, returning the report and whether
    /// the stored state changed. Returns `None` when the report is missing.
    pub async fn set_resolved(
        &self,
        id: i64,
        resolved: bool,
    ) -> Result<Option<ResolveOutcome>, StoreError> {
        let mut tx = self.db.begin().await?;
        let result = sqlx::query("UPDATE reports SET resolved = ? WHERE id = ? AND resolved <> ?")
            .bind(resolved)
            .bind(id)
            .bind(resolved)
            .execute(&mut *tx)
            .await?;
        let row = sqlx::query_as::<_, ReportRow>(&format!(
            "SELECT {REPORT_COLUMNS} FROM {REPORT_JOIN} WHERE r.id = ?"
        ))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;

        let Some(report) = row.map(Report::from) else {
            return Ok(None);
        };

        Ok(Some(ResolveOutcome {
            report,
            changed: result.rows_affected() > 0,
        }))
    }

    /// Delete a report, returning the deleted ID and title when it existed.
    pub async fn delete_report(&self, id: i64) -> Result<Option<DeletedReport>, StoreError> {
        let row = sqlx::query_as::<_, (i64, String)>(
            "DELETE FROM reports WHERE id = ? RETURNING id, title",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(|(id, title)| DeletedReport { id, title }))
    }
}

#[derive(Debug, FromRow)]
struct ReportRow {
    id: i64,
    user_id: String,
    user_name: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
    title: String,
    steps: String,
    details: String,
    resolved: bool,
    created_at: String,
}

impl From<ReportRow> for Report {
    fn from(row: ReportRow) -> Self {
        Self::new(
            row.id,
            row.user_id,
            row.user_name,
            row.display_name,
            row.avatar_url,
            row.title,
            row.steps,
            row.details,
            row.resolved,
            row.created_at,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::StoreError;
    use crate::reports::NewReport;

    fn report<'a>(title: &'a str, user_id: &'a str) -> NewReport<'a> {
        NewReport::new(user_id, title, "open the page", "nothing happened")
    }

    #[tokio::test]
    async fn reporters_are_resolved_from_tracked_users() -> Result<(), StoreError> {
        let database = crate::memory().await;
        database
            .users()
            .join(
                "42",
                "alice",
                "Alice",
                false,
                Some("https://cdn.example/avatar.png"),
            )
            .await?;
        let reports = database.reports();

        reports.create_report(&report("Tracked", "42")).await?;
        reports.create_report(&report("Untracked", "99")).await?;

        let page = reports.list_reports(1, 20, None, None).await?;
        let tracked = page.reports.iter().find(|report| report.user_id == "42");
        assert_eq!(
            tracked.and_then(|report| report.user_name.as_deref()),
            Some("alice")
        );
        assert_eq!(
            tracked.and_then(|report| report.display_name.as_deref()),
            Some("Alice")
        );
        assert_eq!(
            tracked.and_then(|report| report.avatar_url.as_deref()),
            Some("https://cdn.example/avatar.png")
        );

        let untracked = page.reports.iter().find(|report| report.user_id == "99");
        assert!(untracked.is_some());
        assert_eq!(
            untracked.and_then(|report| report.user_name.as_deref()),
            None
        );
        assert_eq!(
            untracked.and_then(|report| report.display_name.as_deref()),
            None
        );
        assert_eq!(
            untracked.and_then(|report| report.avatar_url.as_deref()),
            None
        );
        Ok(())
    }

    #[tokio::test]
    async fn resolved_filter_narrows_the_list() -> Result<(), StoreError> {
        let reports = crate::memory().await.reports();
        let first = reports.create_report(&report("First", "1")).await?;
        reports.create_report(&report("Second", "2")).await?;
        reports.set_resolved(first.id, true).await?;

        let resolved = reports.list_reports(1, 20, None, Some(true)).await?;
        let open = reports.list_reports(1, 20, None, Some(false)).await?;
        let all = reports.list_reports(1, 20, None, None).await?;

        assert_eq!(resolved.total, 1);
        assert_eq!(resolved.reports[0].title, "First");
        assert_eq!(open.total, 1);
        assert_eq!(open.reports[0].title, "Second");
        assert_eq!(all.total, 2);
        Ok(())
    }

    #[tokio::test]
    async fn created_reports_are_unresolved_and_listed_newest_first() -> Result<(), StoreError> {
        let reports = crate::memory().await.reports();

        let first = reports.create_report(&report("First", "1")).await?;
        let second = reports.create_report(&report("Second", "2")).await?;

        assert!(!first.resolved);
        assert!(!first.created_at.is_empty());
        assert!(second.id > first.id);

        let page = reports.list_reports(1, 20, None, None).await?;

        assert_eq!(page.total, 2);
        assert_eq!(page.reports[0].title, "Second");
        assert_eq!(page.reports[1].title, "First");
        Ok(())
    }

    #[tokio::test]
    async fn search_matches_title_steps_details_and_user() -> Result<(), StoreError> {
        let reports = crate::memory().await.reports();
        reports.create_report(&report("Login broken", "42")).await?;
        reports
            .create_report(&NewReport::new("7", "Unrelated", "clicked login", "crash"))
            .await?;

        let by_title = reports.list_reports(1, 20, Some("login"), None).await?;
        let by_user = reports.list_reports(1, 20, Some("42"), None).await?;
        let by_details = reports.list_reports(1, 20, Some("crash"), None).await?;
        let missing = reports.list_reports(1, 20, Some("absent"), None).await?;
        let percent = reports.list_reports(1, 20, Some("%"), None).await?;
        let underscore = reports.list_reports(1, 20, Some("_"), None).await?;

        assert_eq!(by_title.total, 2);
        assert_eq!(by_user.total, 1);
        assert_eq!(by_details.total, 1);
        assert_eq!(missing.total, 0);
        assert_eq!(percent.total, 0);
        assert_eq!(underscore.total, 0);
        Ok(())
    }

    #[tokio::test]
    async fn pagination_returns_the_requested_slice() -> Result<(), StoreError> {
        let reports = crate::memory().await.reports();
        for title in ["One", "Two", "Three"] {
            reports.create_report(&report(title, "1")).await?;
        }

        let page = reports.list_reports(2, 2, None, None).await?;

        assert_eq!(page.total, 3);
        assert_eq!(page.page, 2);
        assert_eq!(page.page_limit, 2);
        assert_eq!(page.reports.len(), 1);
        assert_eq!(page.reports[0].title, "One");
        Ok(())
    }

    #[tokio::test]
    async fn resolve_round_trips_and_unknown_ids_are_none() -> Result<(), StoreError> {
        let reports = crate::memory().await.reports();
        let report = reports.create_report(&report("Broken", "1")).await?;

        let resolved = reports.set_resolved(report.id, true).await?;
        assert_eq!(
            resolved.map(|outcome| (outcome.report.resolved, outcome.changed)),
            Some((true, true))
        );

        let unchanged = reports.set_resolved(report.id, true).await?;
        assert_eq!(
            unchanged.map(|outcome| (outcome.report.resolved, outcome.changed)),
            Some((true, false))
        );

        let unresolved = reports.set_resolved(report.id, false).await?;
        assert_eq!(
            unresolved.map(|outcome| (outcome.report.resolved, outcome.changed)),
            Some((false, true))
        );

        assert!(reports.set_resolved(report.id + 1, true).await?.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn stats_count_open_and_resolved_reports() -> Result<(), StoreError> {
        let reports = crate::memory().await.reports();
        let first = reports.create_report(&report("First", "1")).await?;
        reports.create_report(&report("Second", "2")).await?;
        reports.set_resolved(first.id, true).await?;

        let stats = reports.report_stats().await?;
        assert_eq!((stats.total, stats.resolved, stats.open), (2, 1, 1));

        reports.delete_report(first.id).await?;
        let stats = reports.report_stats().await?;
        assert_eq!((stats.total, stats.resolved, stats.open), (1, 0, 1));
        Ok(())
    }

    #[tokio::test]
    async fn delete_removes_only_existing_reports() -> Result<(), StoreError> {
        let reports = crate::memory().await.reports();
        let report = reports.create_report(&report("Broken", "1")).await?;

        assert!(reports.delete_report(report.id).await?.is_some());
        assert!(reports.delete_report(report.id).await?.is_none());
        assert_eq!(reports.list_reports(1, 20, None, None).await?.total, 0);
        Ok(())
    }
}
