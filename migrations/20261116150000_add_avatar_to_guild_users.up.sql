-- shortcut: users who left before this migration keep avatar_url NULL until they rejoin; avoid a one-request-per-row backfill.
ALTER TABLE guild_users ADD COLUMN avatar_url TEXT;
