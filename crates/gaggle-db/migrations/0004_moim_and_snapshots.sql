CREATE TABLE IF NOT EXISTS model_instructions (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    instruction TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS chat_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    message_count INTEGER NOT NULL DEFAULT 0,
    summary TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_model_instructions_chat ON model_instructions(chat_id);
CREATE INDEX IF NOT EXISTS idx_chat_snapshots_chat ON chat_snapshots(chat_id);