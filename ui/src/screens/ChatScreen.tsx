import React, { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";
import Markdown from "../components/Markdown";
import { Button } from "../components/ui/Button";
import { AutoTextarea } from "../components/ui/AutoTextarea";
import { ToolCallCard } from "../components/ToolCallCard";

export default function ChatScreen() {
  const { projectId, setProjectId, chatId, setChatId } = useApp();
  const [projects, setProjects] = useState<T.ProjectStatus[]>([]);
  const [chats, setChats] = useState<T.Chat[]>([]);
  const [chat, setChat] = useState<T.Chat | null>(null);
  const [messages, setMessages] = useState<T.Message[]>([]);
  const [parts, setParts] = useState<T.MessagePart[]>([]);
  const [models, setModels] = useState<T.Model[]>([]);
  const [modelKey, setModelKey] = useState<string>("local-mock");
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [newProjectName, setNewProjectName] = useState("");
  const [newProjectPath, setNewProjectPath] = useState("");

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

  useEffect(() => {
    if (!chatId) {
      setChat(null);
      setMessages([]);
      setParts([]);
      return;
    }
    api.chats.get(chatId).then(setChat).catch(() => {});
    api.chats.messages(chatId).then((r) => {
      setMessages(r.messages);
      setParts(r.message_parts);
    }).catch(() => {});
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

  const send = async () => {
    if (!chatId || !draft.trim() || sending) return;
    const prompt = draft;
    setDraft("");
    setSending(true);
    setError(null);
    try {
      const outcome = await api.chats.submit(chatId, prompt, modelKey);
      if (outcome.outcome === "failed") {
        setError(outcome.error || "Run failed");
      }
      const r = await api.chats.messages(chatId);
      setMessages(r.messages);
      setParts(r.message_parts);
      if (outcome.run) {
        api.chats.get(chatId).then(setChat).catch(() => {});
      }
    } catch (e) {
      setError(String(e));
      setDraft(prompt);
    } finally {
      setSending(false);
    }
  };

  const partsFor = (msgId: string) => parts.filter((p) => p.message_id === msgId);

  if (!projects.length) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4 p-6">
        <h2 className="text-xl font-semibold text-text-primary">Welcome to Gaggle</h2>
        <p className="text-text-secondary">Register a project to start chatting.</p>
        <div className="w-full max-w-md bg-background-secondary border border-border-primary rounded-xl p-4 flex flex-col gap-2">
          <h3 className="font-medium text-text-primary">New Project</h3>
          <input className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm" placeholder="Project name" value={newProjectName} onChange={(e) => setNewProjectName(e.target.value)} />
          <input className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm" placeholder="Root path (absolute)" value={newProjectPath} onChange={(e) => setNewProjectPath(e.target.value)} />
          <Button variant="primary" onClick={createProject}>Create Project</Button>
          {error && <div className="text-text-danger text-sm">{error}</div>}
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full min-h-0">
      <div className="flex items-center gap-2 px-4 py-2 border-b border-border-primary bg-background-primary">
        <select className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm" value={projectId || ""} onChange={(e) => setProjectId(e.target.value)}>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>{p.name}</option>
          ))}
        </select>
        <select className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm" value={chatId || ""} onChange={(e) => setChatId(e.target.value || null)}>
          <option value="">— select chat —</option>
          {chats.map((c) => (
            <option key={c.id} value={c.id}>{c.title}</option>
          ))}
        </select>
        <Button variant="secondary" size="sm" onClick={createChat}>+ New Chat</Button>
        <div className="flex-1" />
        <select className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm" value={modelKey} onChange={(e) => setModelKey(e.target.value)}>
          <option value="local-mock">local-mock</option>
          {models.map((m) => (
            <option key={m.id} value={m.model_key}>{m.display_name}</option>
          ))}
        </select>
      </div>
      <div className="flex-1 overflow-y-auto p-6">
        {!chatId && <div className="text-center text-text-secondary py-12">Select or create a chat to begin.</div>}
        {chatId && messages.length === 0 && (
          <div className="text-center text-text-secondary py-12">
            <h2 className="text-lg font-semibold text-text-primary">{chat?.title || "New Chat"}</h2>
            <p>Send a message to start the conversation.</p>
          </div>
        )}
        {messages.map((m) => (
          <div key={m.id} className="relative mt-4">
            <div className="text-xs uppercase tracking-wide text-text-tertiary mb-1">{m.role}</div>
            {m.role === "user" ? (
              <div className="message flex justify-end w-full">
                <div className="flex-col max-w-[85%] w-fit">
                  <div className="user-message-bubble flex bg-text-primary text-background-primary rounded-xl py-2.5 px-4">
                    {partsFor(m.id).map((p) => {
                      if (p.kind === "text") return <div key={p.id}>{p.content}</div>;
                      return null;
                    })}
                  </div>
                </div>
              </div>
            ) : (
              <div className="goose-message flex w-[90%] justify-start min-w-0">
                <div className="flex flex-col w-full min-w-0">
                  <div className="agent-message-bubble w-full">
                    {partsFor(m.id).map((p) => {
                      if (p.kind === "text") return <Markdown key={p.id} text={p.content || ""} />;
                      if (p.kind === "tool_call") {
                        return <ToolCallCard key={p.id} kind="tool_call" content={p.content} />;
                      }
                      if (p.kind === "tool_result") {
                        return <ToolCallCard key={p.id} kind="tool_result" content={p.content} />;
                      }
                      if (p.kind === "reasoning") {
                        return (
                          <details key={p.id} className="mt-2 text-sm text-text-secondary">
                            <summary>Reasoning</summary>
                            <pre className="text-xs overflow-x-auto">{p.content}</pre>
                          </details>
                        );
                      }
                      return null;
                    })}
                  </div>
                </div>
              </div>
            )}
          </div>
        ))}
        {sending && <div className="text-text-secondary text-sm">Generating response…</div>}
        {error && <div className="text-text-danger text-sm mt-2">{error}</div>}
      </div>
      {chatId && (
        <div className="border-t border-border-primary bg-background-primary">
          <div className="px-3 pt-3">
            <AutoTextarea
              className="flex-1"
              minRows={1}
              maxRows={10}
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              placeholder="Message Gaggle… (Enter to send, Shift+Enter for newline)"
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  send();
                }
              }}
              disabled={sending}
            />
          </div>
          <div className="flex items-center gap-2 px-3 py-2">
            <select className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-xs" value={modelKey} onChange={(e) => setModelKey(e.target.value)}>
              <option value="local-mock">local-mock</option>
              {models.map((m) => (
                <option key={m.id} value={m.model_key}>{m.display_name}</option>
              ))}
            </select>
            <div className="flex-1" />
            <Button variant="primary" size="sm" onClick={send} disabled={sending || !draft.trim()}>
              Send
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}