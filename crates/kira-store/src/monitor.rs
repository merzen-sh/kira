//! Host resource usage samples.

use kira_protocol::MonitorSample;
use sqlx::FromRow;

use crate::{Database, StoreError};

/// Seconds of samples kept before old rows are deleted.
pub const RETENTION_SECONDS: i64 = 24 * 60 * 60;

const SAMPLE_COLUMNS: &str =
    "recorded_at, cpu_percent, memory_used, memory_total, disk_used, disk_total";

/// Persistence for host resource usage samples.
#[derive(Clone)]
pub struct MonitorRepository {
    db: Database,
}

impl MonitorRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    /// Store a sample and delete records older than the retention window.
    pub async fn record(&self, sample: &MonitorSample) -> Result<(), StoreError> {
        sqlx::query(&format!(
            "INSERT INTO monitor_samples ({SAMPLE_COLUMNS}) VALUES (?, ?, ?, ?, ?, ?)"
        ))
        .bind(sample.recorded_at)
        .bind(sample.cpu_percent)
        .bind(as_i64(sample.memory_used))
        .bind(as_i64(sample.memory_total))
        .bind(as_i64(sample.disk_used))
        .bind(as_i64(sample.disk_total))
        .execute(self.db.pool())
        .await?;

        sqlx::query("DELETE FROM monitor_samples WHERE recorded_at < unixepoch('now') - ?")
            .bind(RETENTION_SECONDS)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Return the newest sample, if any.
    pub async fn latest(&self) -> Result<Option<MonitorSample>, StoreError> {
        let row = sqlx::query_as::<_, MonitorRow>(&format!(
            "SELECT {SAMPLE_COLUMNS} FROM monitor_samples ORDER BY recorded_at DESC, id DESC LIMIT 1"
        ))
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(MonitorSample::from))
    }

    /// Return samples since `since`, averaged into `bucket` second groups.
    pub async fn history(&self, since: i64, bucket: i64) -> Result<Vec<MonitorSample>, StoreError> {
        let bucket = bucket.max(1);
        let rows = sqlx::query_as::<_, MonitorRow>(
            "SELECT MIN(recorded_at) AS recorded_at, AVG(cpu_percent) AS cpu_percent, \
             CAST(AVG(memory_used) AS INTEGER) AS memory_used, MAX(memory_total) AS memory_total, \
             CAST(AVG(disk_used) AS INTEGER) AS disk_used, MAX(disk_total) AS disk_total \
             FROM monitor_samples WHERE recorded_at >= ? GROUP BY recorded_at / ? \
             ORDER BY recorded_at",
        )
        .bind(since)
        .bind(bucket)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(MonitorSample::from).collect())
    }
}

#[derive(Debug, FromRow)]
struct MonitorRow {
    recorded_at: i64,
    cpu_percent: f64,
    memory_used: i64,
    memory_total: i64,
    disk_used: i64,
    disk_total: i64,
}

impl From<MonitorRow> for MonitorSample {
    fn from(row: MonitorRow) -> Self {
        Self::new(
            row.recorded_at,
            row.cpu_percent,
            u64::try_from(row.memory_used).unwrap_or_default(),
            u64::try_from(row.memory_total).unwrap_or_default(),
            u64::try_from(row.disk_used).unwrap_or_default(),
            u64::try_from(row.disk_total).unwrap_or_default(),
        )
    }
}

fn as_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory;

    fn sample(recorded_at: i64, cpu_percent: f64) -> MonitorSample {
        MonitorSample::new(recorded_at, cpu_percent, 100, 200, 300, 400)
    }

    fn now() -> i64 {
        i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_secs(),
        )
        .expect("timestamp fits i64")
    }

    #[tokio::test]
    async fn history_is_averaged_into_buckets() -> Result<(), StoreError> {
        let monitor = memory().await.monitor();
        let now = now();

        monitor.record(&sample(now - 90, 10.0)).await?;
        monitor.record(&sample(now - 30, 20.0)).await?;
        monitor.record(&sample(now - 20, 30.0)).await?;

        let history = monitor.history(now - 120, 60).await?;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].cpu_percent, 10.0);
        assert_eq!(history[1].cpu_percent, 25.0);
        Ok(())
    }

    #[tokio::test]
    async fn recording_prunes_samples_older_than_a_day() -> Result<(), StoreError> {
        let monitor = memory().await.monitor();
        let now = now();

        monitor
            .record(&sample(now - RETENTION_SECONDS - 60, 99.0))
            .await?;
        monitor.record(&sample(now, 5.0)).await?;

        let history = monitor.history(0, RETENTION_SECONDS).await?;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].cpu_percent, 5.0);

        let latest = monitor.latest().await?.expect("a recent sample");
        assert_eq!(latest.recorded_at, now);
        Ok(())
    }
}
