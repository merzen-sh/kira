-- shortcut: rows that already left before this migration stay is_bot = 0 until they rejoin; a Discord backfill costs one request per row.
ALTER TABLE guild_users ADD COLUMN is_bot INTEGER NOT NULL DEFAULT 0;
