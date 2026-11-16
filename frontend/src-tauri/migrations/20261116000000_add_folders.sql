-- Folders for organising meetings (a meeting is in at most one folder).
CREATE TABLE IF NOT EXISTS folders (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL
);

ALTER TABLE meetings ADD COLUMN folder_id TEXT;
