CREATE TABLE monitor_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    recorded_at INTEGER NOT NULL,
    cpu_percent REAL NOT NULL,
    memory_used INTEGER NOT NULL,
    memory_total INTEGER NOT NULL,
    disk_used INTEGER NOT NULL,
    disk_total INTEGER NOT NULL
);

CREATE INDEX monitor_samples_recorded_at ON monitor_samples (recorded_at);
