CREATE TABLE guild_users (
    user_id TEXT PRIMARY KEY,
    user_name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'left')),
    join_at TEXT NOT NULL,
    leave_at TEXT,
    last_synced_at TEXT,
    next_sync_at TEXT
);

CREATE INDEX guild_users_status_sync ON guild_users (status, next_sync_at);

CREATE TABLE guild_user_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK (event_type IN ('join', 'leave')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX guild_user_events_user ON guild_user_events (user_id, id);

CREATE TABLE guild_sync_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    running INTEGER NOT NULL DEFAULT 0,
    cursor TEXT NOT NULL DEFAULT '',
    locked_at TEXT,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO guild_sync_state (id) VALUES (1);
