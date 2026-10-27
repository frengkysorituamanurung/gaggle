import React, { useEffect, useRef, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import { subscribeToRunEvents } from "../lib/runEvents";
import type * as T from "../types";
import Markdown from "../components/Markdown";
import { Button } from "../components/ui/Button";
import { ChatInput } from "../components/ChatInput";
import { ToolCallCard } from "../components/ToolCallCard";
import { ToolApprovalCard } from "../components/ToolApprovalCard";
import { ThinkingBlock } from "../components/ThinkingBlock";
import { defineMessages, useIntl } from "../i18n";

const messages = defineMessages({
  title: { id: "chat.title", defaultMessage: "Chat" },
});

type ToolEventState = {
  tool_call_id: string;
  tool_name?: string;
  label?: string;
  status?: string;
  duration_ms?: number;
  error?: string;
  result_bytes?: number;
};

export default function ChatScreen() {
  const intl = useIntl();
  const { projectId, setProjectId, chatId, setChatId } = useApp();
  const [projects, setProjects] = useState<T.ProjectStatus[]>([]);
  const [chats, setChats] = useState<T.Chat[]>([]);
  const [chat, setChat] = useState<T.Chat | null>(null);
  const [messagesList, setMessagesList] = useState<T.Message[]>([]);
  const [parts, setParts] = useState<T.MessagePart[]>([]);
  const [models, setModels] = useState<T.Model[]>([]);
  const [modelKey, setModelKey] = useState<string>("local-mock");
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [streamingText, setStreamingText] = useState("");
  const [currentRunId, setCurrentRunId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [toolEvents, setToolEvents] = useState<Record<string, ToolEventState>>({});
  const [pendingApproval, setPendingApproval] = useState<T.Approval | null>(null);
  const [runStatus, setRunStatus] = useState<string | null>(null);
  const [newProjectName, setNewProjectName] = useState("");
  const [newProjectPath, setNewProjectPath] = useState("");

  const unsubscribeRef = useRef<(() => void) | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const refreshProjects = async () => {
    const ps = await api.projects.list();
    setProjects(ps);
    if (!projectId && ps.length) setProjectId(ps[0].id);
  };

  useEffect(() => {
    refreshProjects().catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    if (!projectId) return;
    api.projectChats(projectId).then(setChats).catch(() => {});
    (async () => {
      const ps = await api.providers.list();
      const allModels: T.Model[] = [];
      for (const p of ps) {
        try {
          const ms = await api.providers.models(p.provider.id);
          allModels.push(...ms);
        } catch {
          /* ignore */
        }
      }
      setModels(allModels);
    })();
  }, [projectId]);

  const loadChat = async (id: string) => {
    try {
      const c = await api.chats.get(id);
      setChat(c);
      const r = await api.chats.messages(id);
      setMessagesList(r.messages);
      setParts(r.message_parts);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    if (!chatId) {
      setChat(null);
      setMessagesList([]);
      setParts([]);
      return;
    }
    loadChat(chatId).catch(() => {});
  }, [chatId]);

  const createProject = async () => {
    if (!newProjectName.trim() || !newProjectPath.trim()) return;
    try {
      const p = await api.projects.create(newProjectName.trim(), newProjectPath.trim());
      setProjects((prev) => [...prev, p]);
      setProjectId(p.id);
      setNewProjectName("");
      setNewProjectPath("");
    } catch (e) {
      setError(String(e));
    }
  };

  const createChat = async () => {
    if (!projectId) return;
    try {
      const c = await api.chats.create({
        project_id: projectId,
        title: "New Chat",
        mode: "local",
        permission_mode: "ask",
      });
      setChats((prev) => [c, ...prev]);
      setChatId(c.id);
    } catch (e) {
      setError(String(e));
    }
  };

  const finalizeRun = async () => {
    unsubscribeRef.current?.();
    unsubscribeRef.current = null;
    setSending(false);
    setCurrentRunId(null);
    setRunStatus(null);
    if (chatId) await loadChat(chatId);
  };

  const send = async () => {
    if (!chatId || !draft.trim() || sending) return;
    const prompt = draft;
    setDraft("");
    setSending(true);
    setError(null);
    setStreamingText("");
    setToolEvents({});
    setPendingApproval(null);
    setRunStatus("running");
    try {
      const { run_id } = await api.chats.startRun(chatId, prompt, modelKey);
      setCurrentRunId(run_id);
      unsubscribeRef.current = subscribeToRunEvents(run_id, {
        "message.delta": (ev) => {
          const text = (ev.payload.text as string) || "";
          setStreamingText((prev) => prev + text);
        },
        "tool.started": (ev) => {
          const id = (ev.payload.tool_call_id as string) || String(Date.now());
          setToolEvents((prev) => ({
            ...prev,
            [id]: {
              tool_call_id: id,
              tool_name: ev.payload.tool_name as string,
              label: (ev.payload.label as string) || (ev.payload.tool_name as string),
              status: "running",
            },
          }));
        },
        "tool.completed": (ev) => {
          const id = (ev.payload.tool_call_id as string) || "";
          setToolEvents((prev) => ({
            ...prev,
            [id]: {
              ...prev[id],
              tool_call_id: id,
              tool_name: prev[id]?.tool_name || (ev.payload.tool_name as string),
              label: prev[id]?.label || (ev.payload.label as string) || (ev.payload.tool_name as string),
              status: "completed",
              duration_ms: ev.payload.duration_ms as number,
            },
          }));
        },
        "tool.failed": (ev) => {
          const id = (ev.payload.tool_call_id as string) || "";
          setToolEvents((prev) => ({
            ...prev,
            [id]: {
              ...prev[id],
              tool_call_id: id,
              tool_name: prev[id]?.tool_name || (ev.payload.tool_name as string),
              label: prev[id]?.label || (ev.payload.tool_name as string),
              status: "failed",
              error: ev.payload.error as string,
            },
          }));
        },
        "approval.requested": (ev) => {
          setPendingApproval({
            id: (ev.payload.approval_id as string) || "",
            tool_call_id: (ev.payload.tool_call_id as string) || "",
            kind: "tool",
            summary: (ev.payload.summary as string) || "",
            command: (ev.payload.command as string) || null,
            cwd: (ev.payload.cwd as string) || null,
            status: "pending",
            decided_by_device: null,
            remember_rule_id: null,
            decided_at: null,
            created_at: Date.now(),
          });
          setRunStatus("waiting_approval");
        },
        "run.completed": async () => {
          setRunStatus("completed");
          await finalizeRun();
        },
        "run.failed": async (ev) => {
          setError((ev.payload.error as string) || "Run failed");
          setRunStatus("failed");
          await finalizeRun();
        },
        "run.cancelled": async () => {
          setRunStatus("cancelled");
          await finalizeRun();
        },
      });
    } catch (e) {
      setError(String(e));
      setSending(false);
      setRunStatus(null);
    }
  };

  const cancel = async () => {
    if (!currentRunId) return;
    try {
      await api.runs.cancel(currentRunId);
    } catch (e) {
      setError(String(e));
    }
  };

  const decideApproval = async (approved: boolean, remember?: { pattern: string; matchKind: string }) => {
    if (!pendingApproval) return;
    try {
      await api.approvals.decide(pendingApproval.id, approved, {
        project_id: projectId || undefined,
        remember_pattern: remember?.pattern,
        remember_match_kind: remember?.matchKind,
      });
      setPendingApproval(null);
      // The backend does not auto-resume the run; reload current state.
      if (chatId) await loadChat(chatId);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    return () => {
      unsubscribeRef.current?.();
    };
  }, []);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [messagesList, streamingText, toolEvents, pendingApproval]);

  const partsByMessage = new Map<string, T.MessagePart[]>();
  for (const p of parts) {
    const list = partsByMessage.get(p.message_id) || [];
    list.push(p);
    partsByMessage.set(p.message_id, list);
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 px-4 py-3 border-b border-border-primary">
        <h1 className="text-lg font-semibold text-text-primary">{intl.formatMessage(messages.title)}</h1>
        <select
          className="ml-2 px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
          value={projectId || ""}
          onChange={(e) => setProjectId(e.target.value)}
        >
          {projects.map((p) => (
            <option key={p.id} value={p.id}>{p.name}</option>
          ))}
        </select>
        <select
          className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
          value={chatId || ""}
          onChange={(e) => setChatId(e.target.value)}
        >
          <option value="">Select chat</option>
          {chats.map((c) => (
            <option key={c.id} value={c.id}>{c.title}</option>
          ))}
        </select>
        <Button size="sm" variant="secondary" onClick={createChat}>New chat</Button>
        <select
          className="ml-auto px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
          value={modelKey}
          onChange={(e) => setModelKey(e.target.value)}
        >
          {models.length === 0 ? <option value="local-mock">local-mock</option> : models.map((m) => (
            <option key={m.id} value={m.model_key}>{m.display_name}</option>
          ))}
        </select>
      </div>

      {projects.length === 0 && (
        <div className="px-4 py-3 border-b border-border-primary flex gap-2 items-center">
          <input
            className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
            placeholder="Project name"
            value={newProjectName}
            onChange={(e) => setNewProjectName(e.target.value)}
          />
          <input
            className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
            placeholder="Project root path"
            value={newProjectPath}
            onChange={(e) => setNewProjectPath(e.target.value)}
          />
          <Button size="sm" onClick={createProject}>Create project</Button>
        </div>
      )}

      <div ref={scrollRef} className="flex-1 min-h-0 overflow-y-auto px-4 py-4 space-y-4">
        {messagesList.map((m) => {
          const messageParts = (partsByMessage.get(m.id) || []).sort((a, b) => a.idx - b.idx);
          return (
            <div key={m.id} className={`flex ${m.role === "user" ? "justify-end" : "justify-start"}`}>
              <div className={`max-w-[85%] rounded-lg px-3 py-2 ${m.role === "user" ? "bg-background-tertiary text-text-primary" : "bg-background-secondary text-text-primary"}`}>
                {messageParts.map((p) => {
                  if (p.kind === "text") {
                    return <Markdown key={p.id} text={p.content || ""} />;
                  }
                  if (p.kind === "reasoning") {
                    return <ThinkingBlock key={p.id} content={p.content} />;
                  }
                  if (p.kind === "tool_call") {
                    let label = "Tool call";
                    try {
                      const parsed = JSON.parse(p.content || "{}");
                      label = parsed.tool_name || parsed.name || label;
                    } catch {
                      /* ignore */
                    }
                    return <ToolCallCard key={p.id} label={label} content={p.content} />;
                  }
                  if (p.kind === "tool_result") {
                    return <ToolCallCard key={p.id} label="Tool result" content={p.content} />;
                  }
                  if (p.kind === "attachment") {
                    return <div key={p.id} className="text-xs text-text-secondary">Attachment: {p.content}</div>;
                  }
                  return null;
                })}
                {messageParts.length === 0 && m.role === "assistant" && (
                  <span className="text-text-secondary text-sm">(empty)</span>
                )}
              </div>
            </div>
          );
        })}

        {streamingText && (
          <div className="flex justify-start">
            <div className="max-w-[85%] rounded-lg px-3 py-2 bg-background-secondary text-text-primary">
              <Markdown text={streamingText} />
            </div>
          </div>
        )}

        {Object.values(toolEvents).length > 0 && (
          <div className="space-y-2">
            {Object.values(toolEvents).map((te) => (
              <ToolCallCard
                key={te.tool_call_id}
                label={te.label || te.tool_name || "Tool"}
                status={te.status}
                durationMs={te.duration_ms}
                content={te.error ? JSON.stringify({ error: te.error }) : undefined}
              />
            ))}
          </div>
        )}

        {pendingApproval && (
          <ToolApprovalCard
            approvalId={pendingApproval.id}
            toolName={undefined}
            summary={pendingApproval.summary}
            command={pendingApproval.command}
            cwd={pendingApproval.cwd}
            onDecide={decideApproval}
          />
        )}

        {error && <div className="text-text-danger text-sm">{error}</div>}
        {runStatus && runStatus !== "running" && (
          <div className="text-xs text-text-tertiary">Status: {runStatus}</div>
        )}
      </div>

      <div className="border-t border-border-primary px-4 py-3">
        <ChatInput
          value={draft}
          onChange={setDraft}
          onSend={send}
          onStop={cancel}
          disabled={!chatId || sending}
          sending={sending}
          canStop={sending && !!currentRunId}
        />
      </div>
    </div>
  );
}
