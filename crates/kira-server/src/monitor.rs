//! Host resource sampling for the system monitor.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kira_protocol::MonitorSample;
use kira_store::Database;
use sysinfo::{Disks, System};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(10);

/// Current Unix timestamp in seconds.
pub(crate) fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or_default()
}

/// Record host CPU, memory, and disk usage until the task is aborted.
pub(crate) async fn run(database: Database) {
    let monitor = database.monitor();
    let mut system = System::new();
    system.refresh_memory();
    system.refresh_cpu_all();
    let mut disks = Disks::new_with_refreshed_list();

    // CPU usage needs two refreshes separated by the minimum interval.
    tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;

    loop {
        system.refresh_cpu_all();
        system.refresh_memory();
        disks.refresh(true);

        let (disk_used, disk_total) = disk_usage(&disks);
        let sample = MonitorSample::new(
            unix_now(),
            f64::from(system.global_cpu_usage()),
            system.used_memory(),
            system.total_memory(),
            disk_used,
            disk_total,
        );

        if let Err(error) = monitor.record(&sample).await {
            tracing::warn!("failed to record system metrics: {error}");
        }

        tokio::time::sleep(SAMPLE_INTERVAL).await;
    }
}

/// Usage of the filesystem containing the current directory, falling back to
/// the first listed disk.
fn disk_usage(disks: &Disks) -> (u64, u64) {
    let cwd = std::env::current_dir().ok();
    let disk = cwd
        .as_deref()
        .and_then(|cwd| {
            disks
                .list()
                .iter()
                .filter(|disk| cwd.starts_with(disk.mount_point()))
                .max_by_key(|disk| disk.mount_point().as_os_str().len())
        })
        .or_else(|| disks.list().first());

    disk.map_or((0, 0), |disk| {
        (
            disk.total_space().saturating_sub(disk.available_space()),
            disk.total_space(),
        )
    })
}
