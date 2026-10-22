-- ============================================================================
-- Gaggle Database Schema (SQLite)
-- Conventions:
-- - PK: UUIDv7 as TEXT
-- - Time: INTEGER (Unix epoch in milliseconds)
-- - JSON: TEXT with CHECK(json_valid(col))
-- - Boolean: INTEGER (0/1) with CHECK(col IN (0, 1))
-- - Enum: TEXT with CHECK(col IN (...))
-- - Soft delete: archived_at / deleted_at (NULL = active)
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Domain 1: Configuration, Workspace, and Extensions
-- ----------------------------------------------------------------------------

CREATE TABLE providers (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('chatgpt', 'openrouter', 'openai', 'anthropic', 'deepseek', 'ollama', 'openai_compat')),
    name TEXT NOT NULL,
    base_url TEXT,
    auth_kind TEXT NOT NULL CHECK(auth_kind IN ('oauth', 'api_key', 'none')),
    secret_ref TEXT,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
    created_at INTEGER NOT NULL
);

CREATE TABLE models (
    id TEXT PRIMARY KEY NOT NULL,
    provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    model_key TEXT NOT NULL,
    display_name TEXT NOT NULL,
    context_window INTEGER NOT NULL,
    supports_tools INTEGER NOT NULL DEFAULT 1 CHECK(supports_tools IN (0, 1)),
    supports_vision INTEGER NOT NULL DEFAULT 0 CHECK(supports_vision IN (0, 1)),
    supports_reasoning INTEGER NOT NULL DEFAULT 0 CHECK(supports_reasoning IN (0, 1)),
    price_in_per_mtok REAL,
    price_out_per_mtok REAL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1))
);

CREATE TABLE projects (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL UNIQUE,
    vcs_kind TEXT NOT NULL DEFAULT 'git' CHECK(vcs_kind IN ('git', 'none')),
    default_branch TEXT,
    default_mode TEXT NOT NULL DEFAULT 'local' CHECK(default_mode IN ('local', 'worktree', 'readonly')),
    default_access TEXT NOT NULL DEFAULT 'ask' CHECK(default_access IN ('ask', 'full')),
    default_model_id TEXT REFERENCES models(id) ON DELETE SET NULL,
    archived_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE project_settings (
    project_id TEXT PRIMARY KEY NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    instructions TEXT,
    ignore_globs TEXT CHECK(ignore_globs IS NULL OR json_valid(ignore_globs)),
    env TEXT CHECK(env IS NULL OR json_valid(env)),
    shell TEXT,
    setup_command TEXT
);

CREATE TABLE devices (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    last_ip TEXT,
    last_seen_at INTEGER,
    revoked_at INTEGER,
    created_at INTEGER NOT NULL
);

CREATE TABLE app_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL CHECK(json_valid(value)),
    updated_at INTEGER NOT NULL
);

CREATE TABLE audit_logs (
    id TEXT PRIMARY KEY NOT NULL,
    actor_kind TEXT NOT NULL CHECK(actor_kind IN ('user', 'agent', 'extension', 'system')),
    actor_id TEXT,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT,
    metadata TEXT CHECK(metadata IS NULL OR json_valid(metadata)),
    created_at INTEGER NOT NULL
);

CREATE TABLE extensions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    kind TEXT NOT NULL CHECK(kind IN ('builtin', 'stdio', 'streamable_http', 'sse', 'wasm')),
    version TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('bundled', 'registry', 'local', 'git')),
    command TEXT,
    args TEXT CHECK(args IS NULL OR json_valid(args)),
    url TEXT,
    env_secret_refs TEXT CHECK(env_secret_refs IS NULL OR json_valid(env_secret_refs)),
    timeout_secs INTEGER NOT NULL DEFAULT 120,
    install_path TEXT,
    manifest TEXT CHECK(manifest IS NULL OR json_valid(manifest)),
    checksum TEXT,
    bundled INTEGER NOT NULL DEFAULT 0 CHECK(bundled IN (0, 1)),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
    installed_at INTEGER NOT NULL
);

CREATE TABLE extension_permissions (
    id TEXT PRIMARY KEY NOT NULL,
    extension_id TEXT NOT NULL REFERENCES extensions(id) ON DELETE CASCADE,
    permission TEXT NOT NULL CHECK(permission IN ('fs.read', 'fs.write', 'shell', 'net', 'browser', 'memory')),
    scope TEXT,
    granted INTEGER NOT NULL DEFAULT 0 CHECK(granted IN (0, 1)),
    granted_at INTEGER
);

CREATE TABLE extension_tools (
    id TEXT PRIMARY KEY NOT NULL,
    extension_id TEXT NOT NULL REFERENCES extensions(id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL,
    description TEXT,
    input_schema TEXT CHECK(input_schema IS NULL OR json_valid(input_schema)),
    is_read_only INTEGER NOT NULL DEFAULT 0 CHECK(is_read_only IN (0, 1)),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
    UNIQUE(extension_id, tool_name)
);

CREATE TABLE extension_kv (
    extension_id TEXT NOT NULL REFERENCES extensions(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL CHECK(json_valid(value)),
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (extension_id, key)
);

-- ----------------------------------------------------------------------------
-- Domain 6: Recipes & Schedules (defined before chats due to FK references)
-- ----------------------------------------------------------------------------

CREATE TABLE recipes (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    source_kind TEXT NOT NULL CHECK(source_kind IN ('inline', 'file', 'github', 'url')),
    source_ref TEXT,
    instructions TEXT,
    prompt TEXT,
    definition TEXT NOT NULL,
    default_model_id TEXT REFERENCES models(id) ON DELETE SET NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE recipe_parameters (
    id TEXT PRIMARY KEY NOT NULL,
    recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    param_type TEXT NOT NULL,
    description TEXT,
    required INTEGER NOT NULL DEFAULT 1 CHECK(required IN (0, 1)),
    default_value TEXT,
    idx INTEGER NOT NULL DEFAULT 0,
    CHECK((required = 1 AND default_value IS NULL) OR (required = 0 AND default_value IS NOT NULL)),
    UNIQUE(recipe_id, name)
);

CREATE TABLE recipe_sub_recipes (
    id TEXT PRIMARY KEY NOT NULL,
    recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    child_recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    values_json TEXT CHECK(values_json IS NULL OR json_valid(values_json)),
    sequential_when_repeated INTEGER NOT NULL DEFAULT 0 CHECK(sequential_when_repeated IN (0, 1)),
    idx INTEGER NOT NULL DEFAULT 0,
    UNIQUE(recipe_id, name)
);

CREATE TABLE recipe_extensions (
    recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    extension_id TEXT NOT NULL REFERENCES extensions(id) ON DELETE CASCADE,
    PRIMARY KEY (recipe_id, extension_id)
);

CREATE TABLE schedules (
    id TEXT PRIMARY KEY NOT NULL,
    recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    cron_expr TEXT NOT NULL,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    param_values TEXT CHECK(param_values IS NULL OR json_valid(param_values)),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
    last_run_at INTEGER,
    last_status TEXT CHECK(last_status IS NULL OR last_status IN ('ok', 'error', 'skipped')),
    next_run_at INTEGER,
    created_at INTEGER NOT NULL
);

-- ----------------------------------------------------------------------------
-- Domain 3: Chat, Durable Run, Tool, and Approval
-- Circular references handled with DEFERRABLE INITIALLY DEFERRED
-- ----------------------------------------------------------------------------

CREATE TABLE chats (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    model_id TEXT REFERENCES models(id) ON DELETE SET NULL,
    worktree_id TEXT,
    parent_chat_id TEXT REFERENCES chats(id) ON DELETE SET NULL,
    parent_tool_call_id TEXT,
    recipe_id TEXT REFERENCES recipes(id) ON DELETE SET NULL,
    schedule_id TEXT REFERENCES schedules(id) ON DELETE SET NULL,
    chat_type TEXT NOT NULL DEFAULT 'user' CHECK(chat_type IN ('user', 'subagent', 'scheduled', 'hidden', 'terminal')),
    title TEXT NOT NULL,
    title_user_set INTEGER NOT NULL DEFAULT 0 CHECK(title_user_set IN (0, 1)),
    mode TEXT NOT NULL DEFAULT 'local' CHECK(mode IN ('local', 'worktree', 'readonly')),
    permission_mode TEXT NOT NULL DEFAULT 'ask' CHECK(permission_mode IN ('full', 'ask', 'smart', 'chat_only')),
    recipe_values TEXT CHECK(recipe_values IS NULL OR json_valid(recipe_values)),
    compact_threshold REAL NOT NULL DEFAULT 0.75,
    system_prompt_override TEXT,
    total_input_tokens INTEGER NOT NULL DEFAULT 0,
    total_output_tokens INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0 CHECK(pinned IN (0, 1)),
    archived_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE runs (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    trigger_message_id TEXT,
    model_id TEXT REFERENCES models(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'queued' CHECK(status IN ('queued', 'running', 'waiting_approval', 'paused', 'completed', 'failed', 'cancelled')),
    resume_count INTEGER NOT NULL DEFAULT 0,
    error TEXT,
    last_heartbeat_at INTEGER,
    started_at INTEGER,
    finished_at INTEGER
);

CREATE TABLE messages (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    run_id TEXT REFERENCES runs(id) ON DELETE SET NULL DEFERRABLE INITIALLY DEFERRED,
    parent_id TEXT REFERENCES messages(id) ON DELETE SET NULL,
    seq INTEGER NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system', 'tool')),
    created_at INTEGER NOT NULL,
    UNIQUE(chat_id, seq)
);

CREATE TABLE attachments (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    mime TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    sha256 TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE code_executions (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    stdout TEXT,
    stderr TEXT,
    memory_limit_mb INTEGER NOT NULL DEFAULT 128,
    timeout_ms INTEGER NOT NULL DEFAULT 30000,
    status TEXT NOT NULL CHECK(status IN ('ok', 'error', 'timeout')),
    started_at INTEGER NOT NULL,
    finished_at INTEGER
);

CREATE TABLE tool_calls (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    code_execution_id TEXT REFERENCES code_executions(id) ON DELETE SET NULL,
    extension_id TEXT REFERENCES extensions(id) ON DELETE SET NULL,
    step_index INTEGER NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('code_mode', 'native', 'extension')),
    tool_name TEXT NOT NULL,
    args TEXT NOT NULL CHECK(json_valid(args)),
    result TEXT CHECK(result IS NULL OR json_valid(result)),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'awaiting_approval', 'running', 'ok', 'error', 'denied')),
    started_at INTEGER,
    duration_ms INTEGER
);

CREATE TABLE message_parts (
    id TEXT PRIMARY KEY NOT NULL,
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    idx INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('text', 'reasoning', 'tool_call', 'tool_result', 'attachment')),
    content TEXT,
    tool_call_id TEXT REFERENCES tool_calls(id) ON DELETE SET NULL,
    attachment_id TEXT REFERENCES attachments(id) ON DELETE SET NULL,
    UNIQUE(message_id, idx)
);

CREATE TABLE chat_summaries (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    summary TEXT NOT NULL,
    covers_until_seq INTEGER NOT NULL,
    tokens_before INTEGER NOT NULL,
    tokens_after INTEGER NOT NULL,
    trigger TEXT NOT NULL CHECK(trigger IN ('auto', 'manual')),
    created_at INTEGER NOT NULL
);

CREATE TABLE run_events (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    seq INTEGER NOT NULL,
    kind TEXT NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    created_at INTEGER NOT NULL,
    UNIQUE(run_id, seq)
);

CREATE TABLE run_checkpoints (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    step_index INTEGER NOT NULL,
    state TEXT NOT NULL CHECK(json_valid(state)),
    created_at INTEGER NOT NULL
);

CREATE TABLE command_rules (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    pattern TEXT NOT NULL,
    match_kind TEXT NOT NULL CHECK(match_kind IN ('exact', 'prefix', 'regex')),
    action TEXT NOT NULL CHECK(action IN ('allow', 'deny', 'ask')),
    created_at INTEGER NOT NULL
);

CREATE TABLE approvals (
    id TEXT PRIMARY KEY NOT NULL,
    tool_call_id TEXT NOT NULL UNIQUE REFERENCES tool_calls(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('shell', 'fs_write', 'net', 'browser')),
    summary TEXT NOT NULL,
    command TEXT,
    cwd TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'approved', 'denied', 'expired')),
    decided_by_device TEXT REFERENCES devices(id) ON DELETE SET NULL,
    remember_rule_id TEXT REFERENCES command_rules(id) ON DELETE SET NULL,
    decided_at INTEGER,
    created_at INTEGER NOT NULL
);

CREATE TABLE usage_records (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    model_id TEXT REFERENCES models(id) ON DELETE SET NULL,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cached_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    latency_ms INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

-- ----------------------------------------------------------------------------
-- Domain 4: Git, Changes Panel, and Project Memory
-- ----------------------------------------------------------------------------

CREATE TABLE worktrees (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chat_id TEXT NOT NULL UNIQUE REFERENCES chats(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    path TEXT NOT NULL,
    branch TEXT NOT NULL,
    base_ref TEXT NOT NULL,
    base_commit TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'merged', 'discarded')),
    created_at INTEGER NOT NULL
);

CREATE TABLE file_changes (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    old_path TEXT,
    change_type TEXT NOT NULL CHECK(change_type IN ('add', 'modify', 'delete', 'rename')),
    additions INTEGER NOT NULL DEFAULT 0,
    deletions INTEGER NOT NULL DEFAULT 0,
    patch TEXT,
    before_sha TEXT,
    after_sha TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'accepted', 'reverted')),
    created_at INTEGER NOT NULL
);

CREATE TABLE review_comments (
    id TEXT PRIMARY KEY NOT NULL,
    file_change_id TEXT NOT NULL REFERENCES file_changes(id) ON DELETE CASCADE,
    author_device_id TEXT REFERENCES devices(id) ON DELETE SET NULL,
    line_start INTEGER NOT NULL,
    line_end INTEGER NOT NULL,
    side TEXT NOT NULL CHECK(side IN ('old', 'new')),
    body TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open', 'sent_to_agent', 'resolved')),
    created_at INTEGER NOT NULL
);

CREATE TABLE memories (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    source_chat_id TEXT REFERENCES chats(id) ON DELETE SET NULL,
    source_run_id TEXT REFERENCES runs(id) ON DELETE SET NULL,
    kind TEXT NOT NULL CHECK(kind IN ('fact', 'preference', 'decision', 'todo', 'summary')),
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0 CHECK(pinned IN (0, 1)),
    confidence REAL NOT NULL DEFAULT 1.0,
    last_used_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER
);

CREATE TABLE memory_tags (
    memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (memory_id, tag)
);

CREATE TABLE memory_embeddings (
    memory_id TEXT PRIMARY KEY NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    embedding_model TEXT NOT NULL,
    dim INTEGER NOT NULL,
    vector BLOB NOT NULL,
    created_at INTEGER NOT NULL
);

-- ----------------------------------------------------------------------------
-- Domain 5: Terminal and Browser Sandbox
-- ----------------------------------------------------------------------------

CREATE TABLE terminal_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chat_id TEXT REFERENCES chats(id) ON DELETE SET NULL,
    worktree_id TEXT REFERENCES worktrees(id) ON DELETE SET NULL,
    cwd TEXT NOT NULL,
    shell TEXT NOT NULL,
    cols INTEGER NOT NULL DEFAULT 80,
    rows INTEGER NOT NULL DEFAULT 24,
    pid INTEGER,
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'exited')),
    exit_code INTEGER,
    started_at INTEGER NOT NULL,
    ended_at INTEGER
);

CREATE TABLE browser_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    run_id TEXT REFERENCES runs(id) ON DELETE SET NULL,
    profile_dir TEXT NOT NULL,
    headless INTEGER NOT NULL DEFAULT 1 CHECK(headless IN (0, 1)),
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'closed', 'crashed')),
    started_at INTEGER NOT NULL,
    ended_at INTEGER
);

CREATE TABLE browser_actions (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES browser_sessions(id) ON DELETE CASCADE,
    tool_call_id TEXT REFERENCES tool_calls(id) ON DELETE SET NULL,
    seq INTEGER NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('navigate', 'click', 'type', 'scroll', 'screenshot', 'extract')),
    url TEXT,
    selector TEXT,
    value TEXT,
    result TEXT CHECK(result IS NULL OR json_valid(result)),
    screenshot_attachment_id TEXT REFERENCES attachments(id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL
);

-- ----------------------------------------------------------------------------
-- Domain 7: Tool Permissions and Chat Extensions
-- ----------------------------------------------------------------------------

CREATE TABLE tool_permissions (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    extension_tool_id TEXT NOT NULL REFERENCES extension_tools(id) ON DELETE CASCADE,
    level TEXT NOT NULL CHECK(level IN ('always_allow', 'ask_before', 'never_allow')),
    updated_at INTEGER NOT NULL
);

CREATE TABLE tool_risk_cache (
    id TEXT PRIMARY KEY NOT NULL,
    tool_name TEXT NOT NULL,
    args_fingerprint TEXT NOT NULL,
    is_read_only INTEGER NOT NULL CHECK(is_read_only IN (0, 1)),
    source TEXT NOT NULL CHECK(source IN ('annotation', 'llm_judge', 'user')),
    model_id TEXT REFERENCES models(id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(tool_name, args_fingerprint)
);

CREATE TABLE chat_extensions (
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    extension_id TEXT NOT NULL REFERENCES extensions(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
    state TEXT CHECK(state IS NULL OR json_valid(state)),
    enabled_at INTEGER NOT NULL,
    PRIMARY KEY (chat_id, extension_id)
);

-- ----------------------------------------------------------------------------
-- Recommended Indexes & FTS5 Tables
-- ----------------------------------------------------------------------------

CREATE INDEX idx_chats_project_updated ON chats(project_id, updated_at DESC);
CREATE INDEX idx_chats_parent_chat ON chats(parent_chat_id);
CREATE INDEX idx_chats_schedule ON chats(schedule_id);

CREATE INDEX idx_runs_active_status ON runs(status) WHERE status IN ('running', 'waiting_approval');
CREATE INDEX idx_tool_calls_run_step ON tool_calls(run_id, step_index);
CREATE INDEX idx_approvals_pending ON approvals(status) WHERE status = 'pending';
CREATE INDEX idx_file_changes_chat_status ON file_changes(chat_id, status);
CREATE INDEX idx_memories_project_kind ON memories(project_id, kind);
CREATE INDEX idx_usage_records_created_at ON usage_records(created_at);

CREATE UNIQUE INDEX idx_tool_permissions_scope_tool
    ON tool_permissions(COALESCE(project_id, ''), extension_tool_id);

CREATE INDEX idx_schedules_enabled_next_run
    ON schedules(next_run_at) WHERE enabled = 1;

-- FTS5 Virtual Tables for full-text search on memories and message parts
CREATE VIRTUAL TABLE memories_fts USING fts5(
    title,
    content,
    content='memories',
    content_rowid='rowid'
);

CREATE TRIGGER memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, title, content) VALUES (new.rowid, new.title, new.content);
END;

CREATE TRIGGER memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, title, content) VALUES('delete', old.rowid, old.title, old.content);
END;

CREATE TRIGGER memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, title, content) VALUES('delete', old.rowid, old.title, old.content);
    INSERT INTO memories_fts(rowid, title, content) VALUES (new.rowid, new.title, new.content);
END;

CREATE VIRTUAL TABLE messages_fts USING fts5(
    content,
    content='message_parts',
    content_rowid='rowid'
);

CREATE TRIGGER message_parts_ai AFTER INSERT ON message_parts WHEN new.content IS NOT NULL BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TRIGGER message_parts_ad AFTER DELETE ON message_parts WHEN old.content IS NOT NULL BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;

CREATE TRIGGER message_parts_au AFTER UPDATE ON message_parts BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content)
        SELECT 'delete', old.rowid, old.content WHERE old.content IS NOT NULL;
    INSERT INTO messages_fts(rowid, content)
        SELECT new.rowid, new.content WHERE new.content IS NOT NULL;
END;
