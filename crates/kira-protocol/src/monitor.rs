//! Host resource monitor response types.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A point-in-time sample of host resource usage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct MonitorSample {
    /// Unix timestamp in seconds when the sample was taken.
    pub recorded_at: i64,
    /// Host-wide CPU utilization from 0 to 100.
    pub cpu_percent: f64,
    /// Used physical memory in bytes.
    pub memory_used: u64,
    /// Total physical memory in bytes.
    pub memory_total: u64,
    /// Used space on the monitored filesystem in bytes.
    pub disk_used: u64,
    /// Total space on the monitored filesystem in bytes.
    pub disk_total: u64,
}

impl MonitorSample {
    /// Build a resource usage sample.
    #[must_use]
    pub fn new(
        recorded_at: i64,
        cpu_percent: f64,
        memory_used: u64,
        memory_total: u64,
        disk_used: u64,
        disk_total: u64,
    ) -> Self {
        Self {
            recorded_at,
            cpu_percent,
            memory_used,
            memory_total,
            disk_used,
            disk_total,
        }
    }
}

/// Current resource usage plus a bucketed history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct MonitorReport {
    /// Requested history range: `1h`, `3h`, or `24h`.
    pub range: String,
    /// Most recent sample, when the monitor has recorded one.
    pub current: Option<MonitorSample>,
    /// Historical samples in chronological order.
    pub history: Vec<MonitorSample>,
}

impl MonitorReport {
    /// Build a monitor report from the latest sample and its history.
    #[must_use]
    pub fn new(
        range: impl Into<String>,
        current: Option<MonitorSample>,
        history: Vec<MonitorSample>,
    ) -> Self {
        Self {
            range: range.into(),
            current,
            history,
        }
    }
}
