CREATE TABLE imported_session_metadata (
    chat_id TEXT PRIMARY KEY NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    source TEXT NOT NULL CHECK(source = 'goose'),
    source_session_id TEXT NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    imported_at INTEGER NOT NULL
);

CREATE TABLE imported_message_metadata (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    source TEXT NOT NULL CHECK(source = 'goose'),
    source_message_id TEXT NOT NULL,
    message_seq INTEGER NOT NULL CHECK(message_seq >= 1),
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    imported_at INTEGER NOT NULL,
    UNIQUE(chat_id, message_seq)
);

CREATE TABLE imported_usage_records (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    source TEXT NOT NULL CHECK(source = 'goose'),
    source_record_id TEXT NOT NULL,
    model_key TEXT,
    input_tokens INTEGER NOT NULL DEFAULT 0 CHECK(input_tokens >= 0),
    output_tokens INTEGER NOT NULL DEFAULT 0 CHECK(output_tokens >= 0),
    total_tokens INTEGER NOT NULL DEFAULT 0 CHECK(total_tokens >= 0),
    cache_read_tokens INTEGER NOT NULL DEFAULT 0 CHECK(cache_read_tokens >= 0),
    cache_write_tokens INTEGER NOT NULL DEFAULT 0 CHECK(cache_write_tokens >= 0),
    cost_usd REAL,
    cost_source TEXT NOT NULL CHECK(cost_source IN ('provider_reported', 'estimated', 'unknown')),
    is_compaction INTEGER NOT NULL DEFAULT 0 CHECK(is_compaction IN (0, 1)),
    created_at INTEGER NOT NULL,
    UNIQUE(chat_id, source, source_record_id)
);

CREATE INDEX idx_imported_message_metadata_chat_seq
    ON imported_message_metadata(chat_id, message_seq);
CREATE INDEX idx_imported_usage_records_chat_created
    ON imported_usage_records(chat_id, created_at);
