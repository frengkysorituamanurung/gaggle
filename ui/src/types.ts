export type Project = {
  id: string;
  name: string;
  root_path: string;
  vcs_kind: string;
  default_branch: string | null;
  default_mode: string;
  default_access: string;
  default_model_id: string | null;
  archived_at: number | null;
  created_at: number;
  updated_at: number;
};

export type ProjectStatus = Project & { path_exists: boolean };

export type Provider = {
  id: string;
  kind: string;
  name: string;
  base_url: string | null;
  auth_kind: string;
  secret_ref: string | null;
  enabled: number;
  created_at: number;
};

export type ProviderStatus = {
  provider: Provider;
  has_configured_secret: boolean;
};

export type Model = {
  id: string;
  provider_id: string;
  model_key: string;
  display_name: string;
  context_window: number;
  supports_tools: number;
  supports_vision: number;
  supports_reasoning: number;
  price_in_per_mtok: number | null;
  price_out_per_mtok: number | null;
  enabled: number;
};

export type Chat = {
  id: string;
  project_id: string;
  model_id: string | null;
  worktree_id: string | null;
  parent_chat_id: string | null;
  parent_tool_call_id: string | null;
  recipe_id: string | null;
  schedule_id: string | null;
  chat_type: string;
  title: string;
  title_user_set: number;
  mode: string;
  permission_mode: string;
  recipe_values: string | null;
  compact_threshold: number;
  system_prompt_override: string | null;
  total_input_tokens: number;
  total_output_tokens: number;
  pinned: number;
  archived_at: number | null;
  created_at: number;
  updated_at: number;
};

export type Message = {
  id: string;
  chat_id: string;
  run_id: string | null;
  parent_id: string | null;
  seq: number;
  role: string;
  created_at: number;
};

export type MessagePart = {
  id: string;
  message_id: string;
  idx: number;
  kind: string;
  content: string | null;
  tool_call_id: string | null;
  attachment_id: string | null;
};

export type Run = {
  id: string;
  chat_id: string;
  trigger_message_id: string | null;
  model_id: string | null;
  status: string;
  resume_count: number;
  error: string | null;
  last_heartbeat_at: number | null;
  started_at: number | null;
  finished_at: number | null;
};

export type Approval = {
  id: string;
  tool_call_id: string;
  kind: string;
  summary: string;
  command: string | null;
  cwd: string | null;
  status: string;
  decided_by_device: string | null;
  remember_rule_id: string | null;
  decided_at: number | null;
  created_at: number;
};

export type Extension = {
  id: string;
  name: string;
  description: string | null;
  kind: string;
  version: string;
  source: string;
  command: string | null;
  args: string | null;
  url: string | null;
  env_secret_refs: string | null;
  timeout_secs: number;
  install_path: string | null;
  manifest: string | null;
  checksum: string | null;
  bundled: number;
  enabled: number;
  installed_at: number;
};

export type ExtensionToolSummary = { tool_name: string; is_read_only: boolean };

export type ExtensionInventoryItem = Extension & {
  tool_count: number;
  tools: ExtensionToolSummary[];
};

export type ExtensionsInventory = {
  recommended_max_active_tools: number;
  active_tool_count: number;
  exceeds_recommended_tool_limit: boolean;
  extensions: ExtensionInventoryItem[];
};

export type Recipe = {
  id: string;
  name: string;
  title: string;
  description: string | null;
  source_kind: string | null;
  source_ref: string | null;
  instructions: string | null;
  prompt: string | null;
  definition: string | null;
  default_model_id: string | null;
  created_at: number;
  updated_at: number;
};

export type Schedule = {
  id: string;
  recipe_id: string;
  project_id: string | null;
  cron_expr: string;
  timezone: string;
  param_values: string | null;
  enabled: number;
  next_run_at: number | null;
  last_run_at: number | null;
  created_at: number;
  updated_at: number;
};

export type Memory = {
  id: string;
  project_id: string | null;
  scope: string;
  kind: string;
  title: string;
  content: string;
  tags: string | null;
  source: string;
  created_at: number;
  updated_at: number;
};

export type SkillSummary = {
  name: string;
  description: string;
  path: string;
};

export type AppSetting = {
  key: string;
  value: string;
  updated_at: number;
};

export type AgentLoopOutcome = {
  outcome: "completed" | "waiting_approval" | "cancelled" | "failed";
  run?: Run;
  final_text?: string;
  steps_executed?: number;
  pending_approval_id?: string;
  tool_call_id?: string;
  error?: string;
};

export type RunEvent = {
  id?: number;
  seq: number;
  kind: string;
  payload: string;
};

export type ChatMessagesResponse = {
  messages: Message[];
  message_parts: MessagePart[];
};

export type CompactChatPreviewResponse = {
  summary: string;
  covers_until_seq: number;
  tokens_before: number;
  tokens_after: number;
};

export type ChatExportBundle = {
  chat: Chat;
  messages: Message[];
  message_parts: MessagePart[];
};

export type FsEntry = {
  relative_path: string;
  is_dir: boolean;
  size_bytes: number;
};

export type GitFileStatus = {
  path: string;
  index_status: string;
  worktree_status: string;
  untracked: boolean;
};

export type GitWorkspaceStatus = {
  is_git_repo: boolean;
  branch: string | null;
  is_clean: boolean;
  changed_files: GitFileStatus[];
};

export type GitDiffOutput = {
  is_git_repo: boolean;
  patch: string;
  truncated: boolean;
};

export type Worktree = {
  id: string;
  project_id: string;
  chat_id: string;
  path: string;
  branch: string;
  base_ref: string;
  base_commit: string;
  status: string;
  created_at: number;
};

export type CommandRule = {
  id: string;
  project_id: string | null;
  pattern: string;
  match_kind: string;
  action: string;
  created_at: number;
};

export type ToolPermissionStatus = {
  extension_tool_id: string;
  extension_name: string;
  tool_name: string;
  is_read_only: boolean;
  level: string | null;
};

export type Health = { status: string; service: string };
export type RunEventKind =
  | "run.started"
  | "run.completed"
  | "run.cancelled"
  | "run.failed"
  | "run.persist"
  | "message.id"
  | "message.role"
  | "message.delta"
  | "tool.started"
  | "tool.completed"
  | "tool.failed"
  | "tool.denied"
  | "approval.requested"
  | "approval.approved";

export type RunEventPayload = Record<string, unknown>;

export type TypedRunEvent = {
  seq: number;
  id?: string;
  kind: RunEventKind | string;
  payload: RunEventPayload;
  raw: string;
};

export type DiagnosticsReport = Record<string, unknown>;

export type HealthStatus = {
  status: string;
  version?: string;
};
