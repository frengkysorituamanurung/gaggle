import type * as T from "../types";

const BASE = "";

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

export async function apiFetch<T = unknown>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init.headers || {}),
    },
  });
  if (!res.ok) {
    let msg = res.statusText;
    try {
      const body = await res.text();
      if (body) msg = body;
    } catch {
      /* ignore */
    }
    throw new ApiError(res.status, msg);
  }
  if (res.status === 204) return undefined as T;
  const ct = res.headers.get("content-type") || "";
  if (!ct.includes("application/json")) return undefined as T;
  return (await res.json()) as T;
}

export const api = {
  health: () => apiFetch<T.Health>("/api/health"),
  settings: {
    list: () => apiFetch<T.AppSetting[]>("/api/settings"),
    upsert: (key: string, value: unknown) =>
      apiFetch<T.AppSetting>("/api/settings", {
        method: "PUT",
        body: JSON.stringify({ key, value }),
      }),
  },
  projects: {
    list: () => apiFetch<T.ProjectStatus[]>("/api/projects"),
    create: (name: string, root_path: string, vcs_kind = "git") =>
      apiFetch<T.ProjectStatus>("/api/projects", {
        method: "POST",
        body: JSON.stringify({ name, root_path, vcs_kind }),
      }),
    update: (id: string, patch: Partial<T.Project>) =>
      apiFetch<T.ProjectStatus>(`/api/projects/${id}`, {
        method: "PATCH",
        body: JSON.stringify(patch),
      }),
    files: (id: string, path = "") =>
      apiFetch<T.FsEntry[]>(`/api/projects/${id}/files${path ? `?path=${encodeURIComponent(path)}` : ""}`),
    gitStatus: (id: string) => apiFetch<T.GitWorkspaceStatus>(`/api/projects/${id}/git-status`),
    diff: (id: string) => apiFetch<T.GitDiffOutput>(`/api/projects/${id}/diff`),
    worktrees: (id: string) => apiFetch<T.Worktree[]>(`/api/projects/${id}/worktrees`),
    commandRules: (id: string) => apiFetch<T.CommandRule[]>(`/api/projects/${id}/command-rules`),
    createCommandRule: (id: string, req: { pattern: string; action: string; match_kind?: string }) =>
      apiFetch<T.CommandRule>(`/api/projects/${id}/command-rules`, {
        method: "POST",
        body: JSON.stringify(req),
      }),
    toolPermissions: (id: string) => apiFetch<T.ToolPermissionStatus[]>(`/api/projects/${id}/tool-permissions`),
    setToolPermission: (id: string, extension_tool_id: string, level: string) =>
      apiFetch<T.ToolPermissionStatus>(`/api/projects/${id}/tool-permissions`, {
        method: "PUT",
        body: JSON.stringify({ extension_tool_id, level }),
      }),
  },
  projectChats: (id: string) => apiFetch<T.Chat[]>(`/api/projects/${id}/chats`),
  chats: {
    create: (req: {
      project_id: string;
      title: string;
      mode?: string;
      permission_mode?: string;
    }) =>
      apiFetch<T.Chat>("/api/chats", { method: "POST", body: JSON.stringify(req) }),
    get: (id: string) => apiFetch<T.Chat>(`/api/chats/${id}`),
    update: (id: string, patch: Partial<T.Chat>) =>
      apiFetch<T.Chat>(`/api/chats/${id}`, {
        method: "PATCH",
        body: JSON.stringify(patch),
      }),
    messages: (id: string) => apiFetch<T.ChatMessagesResponse>(`/api/chats/${id}/messages`),
    submit: (id: string, prompt: string, model_key?: string) =>
      apiFetch<T.AgentLoopOutcome>(`/api/chats/${id}/messages`, {
        method: "POST",
        body: JSON.stringify({ prompt, model_key }),
      }),
    compact: (id: string, trigger?: string) =>
      apiFetch<T.CompactChatPreviewResponse | T.Chat>(`/api/chats/${id}/compact`, {
        method: "POST",
        body: JSON.stringify({ trigger }),
      }),
    compactPreview: (id: string, trigger?: string) =>
      apiFetch<T.CompactChatPreviewResponse>(`/api/chats/${id}/compact/preview`, {
        method: "POST",
        body: JSON.stringify({ trigger }),
      }),
    export: (id: string) => apiFetch<T.ChatExportBundle>(`/api/chats/${id}/export`),
    startRun: (id: string, prompt: string, model_key?: string) =>
      apiFetch<{ run_id: string }>(`/api/chats/${id}/runs`, {
        method: "POST",
        body: JSON.stringify({ prompt, model_key }),
      }),
  },
  terminal: {
    list: () => apiFetch<any[]>("/api/terminal-sessions"),
    create: (req: Record<string, unknown> = {}) =>
      apiFetch<any>("/api/terminal-sessions", { method: "POST", body: JSON.stringify(req) }),
    get: (id: string) => apiFetch<any>(`/api/terminal-sessions/${id}`),
    close: (id: string) => apiFetch<void>(`/api/terminal-sessions/${id}`, { method: "DELETE" }),
    input: (id: string, input: string) =>
      apiFetch<void>(`/api/terminal-sessions/${id}/input`, { method: "POST", body: JSON.stringify({ input }) }),
    resize: (id: string, cols: number, rows: number) =>
      apiFetch<void>(`/api/terminal-sessions/${id}/resize`, { method: "POST", body: JSON.stringify({ cols, rows }) }),
    scrollback: (id: string) => apiFetch<string>(`/api/terminal-sessions/${id}/scrollback`),
    streamUrl: (id: string) => `/api/terminal-sessions/${id}/stream`,
  },
  worktrees: {
    gitStatus: (id: string) => apiFetch<T.GitWorkspaceStatus>(`/api/worktrees/${id}/git-status`),
    diff: (id: string) => apiFetch<T.GitDiffOutput>(`/api/worktrees/${id}/diff`),
    merge: (id: string) => apiFetch<void>(`/api/worktrees/${id}/merge`, { method: "POST" }),
    discard: (id: string) => apiFetch<void>(`/api/worktrees/${id}/discard`, { method: "POST" }),
  },
  runs: {
    cancel: (id: string) => apiFetch<T.Run>(`/api/runs/${id}/cancel`, { method: "POST" }),
    eventsUrl: (id: string, afterSeq = 0) =>
      `/api/runs/${id}/events?after_seq=${afterSeq}`,
  },
  approvals: {
    pending: () => apiFetch<T.Approval[]>("/api/approvals/pending"),
    decide: (
      id: string,
      approved: boolean,
      options?: {
        decided_by_device?: string;
        project_id?: string;
        remember_pattern?: string;
        remember_match_kind?: string;
      },
    ) =>
      apiFetch<T.Approval>(`/api/approvals/${id}/decide`, {
        method: "POST",
        body: JSON.stringify({ approved, ...(options || {}) }),
      }),
  },
  providers: {
    list: () => apiFetch<T.ProviderStatus[]>("/api/providers"),
    create: (req: {
      kind: string;
      name: string;
      base_url?: string;
      auth_kind: string;
      api_key?: string;
    }) =>
      apiFetch<T.ProviderStatus>("/api/providers", {
        method: "POST",
        body: JSON.stringify(req),
      }),
    models: (id: string) => apiFetch<T.Model[]>(`/api/providers/${id}/models`),
    refreshModels: (id: string) => apiFetch<T.Model[]>(`/api/providers/${id}/models/refresh`, { method: "POST" }),
    catalog: (id: string) => apiFetch<string[]>(`/api/providers/${id}/catalog`),
    startChatgptOAuth: (id: string) => apiFetch<{ authorization_url: string }>(`/api/providers/${id}/oauth/chatgpt/start`, { method: "POST", body: "{}" }),
    createModel: (id: string, req: {
      model_key: string;
      display_name: string;
      context_window: number;
      supports_tools?: boolean;
      supports_vision?: boolean;
      supports_reasoning?: boolean;
      price_in_per_mtok?: number | null;
      price_out_per_mtok?: number | null;
    }) =>
      apiFetch<T.Model>(`/api/providers/${id}/models`, {
        method: "POST",
        body: JSON.stringify(req),
      }),
  },
  skills: () => apiFetch<T.SkillSummary[]>("/api/skills"),
  extensions: {
    inventory: () => apiFetch<T.ExtensionsInventory>("/api/extensions/inventory"),
    setEnabled: (id: string, enabled: boolean) =>
      apiFetch<T.Extension>(`/api/extensions/${id}`, {
        method: "PATCH",
        body: JSON.stringify({ enabled: enabled ? 1 : 0 }),
      }),
    installBundled: () => apiFetch<void>("/api/extensions/bundled/install", { method: "POST" }),
    discover: (id: string) => apiFetch<T.Extension>(`/api/extensions/${id}/discover`, { method: "POST" }),
    rediscover: (id: string) => apiFetch<T.Extension>(`/api/extensions/${id}/rediscover`, { method: "POST" }),
  },
  recipes: () => apiFetch<T.Recipe[]>("/api/recipes"),
  recipe: {
    get: (id: string) => apiFetch<T.Recipe>(`/api/recipes/${id}`),
    create: (req: Partial<T.Recipe> & { name: string; title: string }) =>
      apiFetch<T.Recipe>("/api/recipes", { method: "POST", body: JSON.stringify(req) }),
    update: (id: string, patch: Partial<T.Recipe>) =>
      apiFetch<T.Recipe>(`/api/recipes/${id}`, { method: "PATCH", body: JSON.stringify(patch) }),
    delete: (id: string) => apiFetch<void>(`/api/recipes/${id}`, { method: "DELETE" }),
    execute: (id: string, params?: Record<string, unknown>) =>
      apiFetch<T.AgentLoopOutcome>(`/api/recipes/${id}/execute`, { method: "POST", body: JSON.stringify(params || {}) }),
  },
  schedules: (project_id?: string) =>
    apiFetch<T.Schedule[]>(
      `/api/schedules${project_id ? `?project_id=${encodeURIComponent(project_id)}` : ""}`,
    ),
  schedule: {
    get: (id: string) => apiFetch<T.Schedule>(`/api/schedules/${id}`),
    create: (req: { recipe_id: string; project_id?: string | null; cron_expr: string; timezone?: string; param_values?: string | null; enabled?: number }) =>
      apiFetch<T.Schedule>("/api/schedules", { method: "POST", body: JSON.stringify(req) }),
    update: (id: string, patch: Partial<T.Schedule>) =>
      apiFetch<T.Schedule>(`/api/schedules/${id}`, { method: "PATCH", body: JSON.stringify(patch) }),
    delete: (id: string) => apiFetch<void>(`/api/schedules/${id}`, { method: "DELETE" }),
  },
  memories: {
    list: (project_id?: string) =>
      apiFetch<T.Memory[]>(
        `/api/memories${project_id ? `?project_id=${encodeURIComponent(project_id)}` : ""}`,
      ),
    create: (req: { project_id: string; kind: string; title: string; content: string; tags?: string[]; pinned?: boolean }) =>
      apiFetch<T.Memory>("/api/memories", { method: "POST", body: JSON.stringify(req) }),
    get: (id: string) => apiFetch<T.Memory>(`/api/memories/${id}`),
    update: (id: string, patch: Partial<T.Memory>) =>
      apiFetch<T.Memory>(`/api/memories/${id}`, { method: "PATCH", body: JSON.stringify(patch) }),
    delete: (id: string) => apiFetch<void>(`/api/memories/${id}`, { method: "DELETE" }),
    export: () => fetch("/api/memories/export").then((r) => r.blob()),
  },
  diagnostics: () => apiFetch<Record<string, unknown>>("/api/diagnostics"),
};
