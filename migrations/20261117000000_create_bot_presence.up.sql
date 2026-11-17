CREATE TABLE bot_presence (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    status TEXT NOT NULL CHECK (status IN ('online', 'idle', 'dnd', 'invisible')),
    activity_kind TEXT CHECK (
        activity_kind IN ('playing', 'watching', 'listening', 'competing', 'custom')
    ),
    activity_text TEXT,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK ((activity_kind IS NULL) = (activity_text IS NULL))
);

INSERT INTO bot_presence (id, status) VALUES (1, 'online');
