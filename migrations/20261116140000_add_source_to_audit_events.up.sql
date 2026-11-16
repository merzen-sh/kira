ALTER TABLE audit_events ADD COLUMN source TEXT NOT NULL DEFAULT 'system';

CREATE INDEX audit_events_source ON audit_events (source);
