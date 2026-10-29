-- no-transaction
PRAGMA foreign_keys = OFF;
BEGIN IMMEDIATE;

CREATE TABLE providers_rebuild (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN (
        'chatgpt', 'openrouter', 'openai', 'anthropic', 'deepseek', 'ollama',
        'openai_compat', 'google', 'xai', 'azure', 'databricks', 'snowflake',
        'mistral', 'groq', 'together', 'cohere', 'perplexity', 'fireworks',
        'cerebras', 'deepinfra'
    )),
    name TEXT NOT NULL,
    base_url TEXT,
    auth_kind TEXT NOT NULL CHECK(auth_kind IN ('oauth', 'api_key', 'none')),
    secret_ref TEXT,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
    created_at INTEGER NOT NULL
);

INSERT INTO providers_rebuild (
    id, kind, name, base_url, auth_kind, secret_ref, enabled, created_at
)
SELECT id, kind, name, base_url, auth_kind, secret_ref, enabled, created_at
FROM providers;

DROP TABLE providers;
ALTER TABLE providers_rebuild RENAME TO providers;
COMMIT;
PRAGMA foreign_keys = ON;
