import React, { useEffect, useRef, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import { subscribeToRunEvents } from "../lib/runEvents";
import type * as T from "../types";
import Markdown from "./Markdown";
import { Button } from "./ui/Button";
import { ChatInput, type ChatAttachment } from "./ChatInput";
import { ToolCallCard } from "./ToolCallCard";
import { ToolApprovalCard } from "./ToolApprovalCard";
import { ThinkingBlock } from "./ThinkingBlock";
import { EmptyState } from "./EmptyState";
import { ErrorState } from "./ErrorState";
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
};

export function BaseChat({
  chatId,
  projectId,
  modelKey,
  onModelKeyChange,
}: {
  chatId: string;
  projectId: string | null;
  modelKey: string;
  onModelKeyChange?: (k: string) => void;
}) {
  const intl = useIntl();
  const { setChatId } = useApp();
  const [chat, setChat] = useState<T.Chat | null>(null);
  const [messagesList, setMessagesList] = useState<T.Message[]>([]);
  const [parts, setParts] = useState<T.MessagePart[]>([]);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [streamingText, setStreamingText] = useState("");
  const [currentRunId, setCurrentRunId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [toolEvents, setToolEvents] = useState<Record<string, ToolEventState>>({});
  const [pendingApproval, setPendingApproval] = useState<T.Approval | null>(null);
  const [runStatus, setRunStatus] = useState<string | null>(null);
  const [attachments, setAttachments] = useState<ChatAttachment[]>([]);
  const [quotedText, setQuotedText] = useState<string | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [mentionItems, setMentionItems] = useState<string[]>([]);
  const [commandHistory, setCommandHistory] = useState<string[]>(() => {
    try {
      const raw = localStorage.getItem("gaggle.commandHistory");
      return raw ? JSON.parse(raw) : [];
    } catch {
      return [];
    }
  });

  const unsubscribeRef = useRef<(() => void) | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);

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

  // Populate mention items from skills + project files.
  useEffect(() => {
    const items: string[] = [];
    api.skills().then((s) => {
      s.forEach((sk) => items.push(sk.name));
      setMentionItems([...items]);
    }).catch(() => {});
    if (projectId) {
      api.projects.files(projectId).then((files) => {
        files.forEach((f) => items.push(f.relative_path));
        setMentionItems([...items]);
      }).catch(() => {});
    }
  }, [projectId]);

  const finalizeRun = async () => {
    unsubscribeRef.current?.();
    unsubscribeRef.current = null;
    setSending(false);
    setCurrentRunId(null);
    setRunStatus(null);
    setAttachments([]);
    if (chatId) await loadChat(chatId);
  };

  const send = async () => {
    if (!chatId || (!draft.trim() && attachments.length === 0) || sending) return;
    let prompt = draft;
    if (attachments.length > 0) {
      const names = attachments.map((a) => a.name).join(", ");
      prompt = prompt ? `${prompt}\n\n[Attachments: ${names}]` : `[Attachments: ${names}]`;
    }
    // Persist command history
    if (draft.trim()) {
      setCommandHistory((prev) => {
        const next = [draft.trim(), ...prev.filter((x) => x !== draft.trim())].slice(0, 50);
        try { localStorage.setItem("gaggle.commandHistory", JSON.stringify(next)); } catch {}
        return next;
      });
    }
    setDraft("");
    setSending(true);
    setError(null);
    setStreamingText("");
    setToolEvents({});
    setPendingApproval(null);
    setRunStatus("running");
    setAttachments([]);
    try {
      const attPayload = attachments.map((a) => ({ name: a.name, content: a.previewUrl || a.name }));
      const { run_id } = await api.chats.startRun(chatId, prompt, modelKey, attPayload);
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
      if (chatId) await loadChat(chatId);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleSlashCommand = (cmd: { trigger: string }) => {
    if (cmd.trigger === "/compact" && chatId) {
      api.chats.compact(chatId).catch((e) => setError(String(e)));
    }
    if (cmd.trigger === "/clear") {
      setDraft("");
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

  const filteredMessages = searchQuery
    ? messagesList.filter((m) => {
        const ms = partsByMessage.get(m.id) || [];
        return ms.some((p) => (p.content || "").toLowerCase().includes(searchQuery.toLowerCase()));
      })
    : messagesList;

  if (!chatId) {
    return <EmptyState title="No chat selected" description="Select or create a chat to begin." />;
  }

  return (
    <div className="flex flex-col h-full">
      {searchOpen && (
        <div className="px-4 py-2 border-b border-border-primary">
          <input
            className="w-full px-3 py-1.5 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
            placeholder="Search in conversation…"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>
      )}

      <div ref={scrollRef} className="flex-1 min-h-0 overflow-y-auto px-4 py-4 space-y-4">
        {filteredMessages.map((m) => {
          const messageParts = (partsByMessage.get(m.id) || []).sort((a, b) => a.idx - b.idx);
          return (
            <div key={m.id} className={`flex ${m.role === "user" ? "justify-end" : "justify-start"}`}>
              <div className={`max-w-[85%] rounded-lg px-3 py-2 ${m.role === "user" ? "bg-background-tertiary text-text-primary" : "bg-background-secondary text-text-primary"}`}>
                {messageParts.map((p) => {
                  if (p.kind === "text") return <Markdown key={p.id} text={p.content || ""} />;
                  if (p.kind === "reasoning") return <ThinkingBlock key={p.id} content={p.content} />;
                  if (p.kind === "tool_call") {
                    let label = "Tool call";
                    try {
                      const parsed = JSON.parse(p.content || "{}");
                      label = parsed.tool_name || parsed.name || label;
                    } catch {}
                    return <ToolCallCard key={p.id} label={label} content={p.content} />;
                  }
                  if (p.kind === "tool_result") return <ToolCallCard key={p.id} label="Tool result" content={p.content} />;
                  if (p.kind === "attachment") return <div key={p.id} className="text-xs text-text-secondary">Attachment: {p.content}</div>;
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
            summary={pendingApproval.summary}
            command={pendingApproval.command}
            cwd={pendingApproval.cwd}
            onDecide={decideApproval}
          />
        )}

        {error && <ErrorState message={error} />}
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
          attachments={attachments}
          onAttachmentsChange={setAttachments}
          mentionItems={mentionItems}
          commandHistory={commandHistory}
          quotedText={quotedText}
          onQuoteConsumed={() => setQuotedText(null)}
          searchOpen={searchOpen}
          onSearchToggle={() => setSearchOpen((v) => !v)}
          onSlashCommand={handleSlashCommand}
        />
      </div>
    </div>
  );
}

export default BaseChat;