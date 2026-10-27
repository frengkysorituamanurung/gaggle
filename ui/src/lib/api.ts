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
  },
  runs: {
    cancel: (id: string) => apiFetch<T.Run>(`/api/runs/${id}/cancel`, { method: "POST" }),
    eventsUrl: (id: string, afterSeq = 0) =>
      `/api/runs/${id}/events?after_seq=${afterSeq}`,
  },
  approvals: {
    pending: () => apiFetch<T.Approval[]>("/api/approvals/pending"),
    decide: (id: string, approved: boolean) =>
      apiFetch<T.Approval>(`/api/approvals/${id}/decide`, {
        method: "POST",
        body: JSON.stringify({ approved }),
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
  },
  skills: () => apiFetch<T.SkillSummary[]>("/api/skills"),
  extensions: {
    inventory: () => apiFetch<T.ExtensionsInventory>("/api/extensions/inventory"),
    setEnabled: (id: string, enabled: boolean) =>
      apiFetch<T.Extension>(`/api/extensions/${id}`, {
        method: "PATCH",
        body: JSON.stringify({ enabled: enabled ? 1 : 0 }),
      }),
  },
  recipes: () => apiFetch<T.Recipe[]>("/api/recipes"),
  schedules: (project_id?: string) =>
    apiFetch<T.Schedule[]>(
      `/api/schedules${project_id ? `?project_id=${encodeURIComponent(project_id)}` : ""}`,
    ),
  memories: {
    list: (project_id?: string) =>
      apiFetch<T.Memory[]>(
        `/api/memories${project_id ? `?project_id=${encodeURIComponent(project_id)}` : ""}`,
      ),
  },
  diagnostics: () => apiFetch<Record<string, unknown>>("/api/diagnostics"),
};