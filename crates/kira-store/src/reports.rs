//! Bug report persistence.

use kira_protocol::{Report, ReportList};
use sqlx::FromRow;

use crate::{Database, StoreError, escape_like};

const REPORT_COLUMNS: &str = "id, user_id, title, steps, details, resolved, created_at";

const FILTER: &str = "(? = '' OR title LIKE ? ESCAPE '\\' OR steps LIKE ? ESCAPE '\\' OR details LIKE ? ESCAPE '\\' OR user_id LIKE ? ESCAPE '\\')";

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
    /// default unresolved state.
    pub async fn create_report(&self, report: &NewReport<'_>) -> Result<Report, StoreError> {
        let sql = format!(
            "INSERT INTO reports (user_id, title, steps, details) \
             VALUES (?, ?, ?, ?) RETURNING {REPORT_COLUMNS}"
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
    /// substring of the title, steps, details, or reporter ID.
    pub async fn list_reports(
        &self,
        page: u32,
        page_limit: u32,
        search: Option<&str>,
    ) -> Result<ReportList, StoreError> {
        let search = search.map(str::trim).filter(|value| !value.is_empty());
        let search = search.unwrap_or_default();
        let pattern = format!("%{}%", escape_like(search));
        let limit = i64::from(page_limit);
        let offset = i64::from(page.saturating_sub(1)) * limit;

        let count_sql = format!("SELECT COUNT(*) FROM reports WHERE {FILTER}");
        let total: i64 = sqlx::query_scalar(&count_sql)
            .bind(search)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .fetch_one(self.db.pool())
            .await?;

        let list_sql = format!(
            "SELECT {REPORT_COLUMNS} FROM reports WHERE {FILTER} \
             ORDER BY id DESC LIMIT ? OFFSET ?"
        );
        let rows = sqlx::query_as::<_, ReportRow>(&list_sql)
            .bind(search)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
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

    /// Set the resolved state of a report, returning it when it exists.
    pub async fn set_resolved(
        &self,
        id: i64,
        resolved: bool,
    ) -> Result<Option<Report>, StoreError> {
        let sql =
            format!("UPDATE reports SET resolved = ? WHERE id = ? RETURNING {REPORT_COLUMNS}");

        let row = sqlx::query_as::<_, ReportRow>(&sql)
            .bind(resolved)
            .bind(id)
            .fetch_optional(self.db.pool())
            .await?;

        Ok(row.map(Report::from))
    }

    /// Delete a report, returning whether it existed.
    pub async fn delete_report(&self, id: i64) -> Result<bool, StoreError> {
        let result = sqlx::query("DELETE FROM reports WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

#[derive(Debug, FromRow)]
struct ReportRow {
    id: i64,
    user_id: String,
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
    async fn created_reports_are_unresolved_and_listed_newest_first() -> Result<(), StoreError> {
        let reports = crate::memory().await.reports();

        let first = reports.create_report(&report("First", "1")).await?;
        let second = reports.create_report(&report("Second", "2")).await?;

        assert!(!first.resolved);
        assert!(!first.created_at.is_empty());
        assert!(second.id > first.id);

        let page = reports.list_reports(1, 20, None).await?;

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

        let by_title = reports.list_reports(1, 20, Some("login")).await?;
        let by_user = reports.list_reports(1, 20, Some("42")).await?;
        let by_details = reports.list_reports(1, 20, Some("crash")).await?;
        let missing = reports.list_reports(1, 20, Some("absent")).await?;
        let percent = reports.list_reports(1, 20, Some("%")).await?;
        let underscore = reports.list_reports(1, 20, Some("_")).await?;

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

        let page = reports.list_reports(2, 2, None).await?;

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
        assert_eq!(resolved.map(|report| report.resolved), Some(true));

        let unresolved = reports.set_resolved(report.id, false).await?;
        assert_eq!(unresolved.map(|report| report.resolved), Some(false));

        assert!(reports.set_resolved(report.id + 1, true).await?.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn delete_removes_only_existing_reports() -> Result<(), StoreError> {
        let reports = crate::memory().await.reports();
        let report = reports.create_report(&report("Broken", "1")).await?;

        assert!(reports.delete_report(report.id).await?);
        assert!(!reports.delete_report(report.id).await?);
        assert_eq!(reports.list_reports(1, 20, None).await?.total, 0);
        Ok(())
    }
}
