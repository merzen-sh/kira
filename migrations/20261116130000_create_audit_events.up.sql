CREATE TABLE audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action TEXT NOT NULL,
    actor_kind TEXT NOT NULL,
    actor_id TEXT,
    actor_name TEXT,
    target_kind TEXT,
    target_id TEXT,
    target_name TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    occurred_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX audit_events_occurred_at ON audit_events (occurred_at DESC, id DESC);
CREATE INDEX audit_events_action ON audit_events (action);
CREATE INDEX audit_events_actor ON audit_events (actor_kind);
