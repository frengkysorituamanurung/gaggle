import React, { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";
import Markdown from "../components/Markdown";

export default function ChatScreen() {
  const { projectId, setProjectId, chatId, setChatId } = useApp();
  const [projects, setProjects] = useState<T.ProjectStatus[]>([]);
  const [chats, setChats] = useState<T.Chat[]>([]);
  const [chat, setChat] = useState<T.Chat | null>(null);
  const [messages, setMessages] = useState<T.Message[]>([]);
  const [parts, setParts] = useState<T.MessagePart[]>([]);
  const [models, setModels] = useState<T.Model[]>([]);
  const [providers, setProviders] = useState<T.ProviderStatus[]>([]);
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
    api.providers.list().then(setProviders).catch(() => {});
  }, []);

  useEffect(() => {
    if (!projectId) return;
    api.projectChats(projectId).then(setChats).catch(() => {});
    // load models from first provider
    (async () => {
      const ps = await api.providers.list();
      setProviders(ps);
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
      // refresh messages
      const r = await api.chats.messages(chatId);
      setMessages(r.messages);
      setParts(r.message_parts);
      if (outcome.run) {
        // refresh chat
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

  const currentProject = projects.find((p) => p.id === projectId);

  if (!projects.length) {
    return (
      <div className="content">
        <h2>Welcome to Gaggle</h2>
        <p className="empty-state">Register a project to start chatting.</p>
        <div className="card" style={{ maxWidth: 480, margin: "0 auto" }}>
          <h3>New Project</h3>
          <div style={{ display: "flex", flexDirection: "column", gap: 8, marginTop: 8 }}>
            <input className="input" placeholder="Project name" value={newProjectName} onChange={(e) => setNewProjectName(e.target.value)} />
            <input className="input" placeholder="Root path (absolute)" value={newProjectPath} onChange={(e) => setNewProjectPath(e.target.value)} />
            <button className="btn primary" onClick={createProject}>Create Project</button>
          </div>
          {error && <div className="error-banner" style={{ marginTop: 8 }}>{error}</div>}
        </div>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="topbar">
        <select className="select" style={{ width: 200 }} value={projectId || ""} onChange={(e) => setProjectId(e.target.value)}>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>{p.name}</option>
          ))}
        </select>
        <select className="select" style={{ width: 220 }} value={chatId || ""} onChange={(e) => setChatId(e.target.value || null)}>
          <option value="">— select chat —</option>
          {chats.map((c) => (
            <option key={c.id} value={c.id}>{c.title}</option>
          ))}
        </select>
        <button className="btn" onClick={createChat}>+ New Chat</button>
        <div style={{ flex: 1 }} />
        <select className="select" style={{ width: 220 }} value={modelKey} onChange={(e) => setModelKey(e.target.value)}>
          <option value="local-mock">local-mock</option>
          {models.map((m) => (
            <option key={m.id} value={m.model_key}>{m.display_name}</option>
          ))}
        </select>
      </div>
      <div className="content">
        {!chatId && <div className="empty-state">Select or create a chat to begin.</div>}
        {chatId && messages.length === 0 && (
          <div className="empty-state">
            <h2>{chat?.title || "New Chat"}</h2>
            <p>Send a message to start the conversation.</p>
          </div>
        )}
        {messages.map((m) => (
          <div key={m.id} className={`message ${m.role}`}>
            <div className="message-role">{m.role}</div>
            {partsFor(m.id).map((p) => {
              if (p.kind === "text") {
                return m.role === "assistant" ? (
                  <Markdown key={p.id} text={p.content || ""} />
                ) : (
                  <div key={p.id} className="message-body">{p.content}</div>
                );
              }
              if (p.kind === "tool_call") {
                return (
                  <div key={p.id} className="card" style={{ marginTop: 8, fontSize: "var(--text-sm)" }}>
                    <strong>Tool call</strong>
                    <pre>{p.content}</pre>
                  </div>
                );
              }
              if (p.kind === "tool_result") {
                return (
                  <div key={p.id} className="card" style={{ marginTop: 8, fontSize: "var(--text-sm)" }}>
                    <strong>Tool result</strong>
                    <pre>{p.content}</pre>
                  </div>
                );
              }
              if (p.kind === "reasoning") {
                return (
                  <details key={p.id} style={{ marginTop: 8, fontSize: "var(--text-sm)", color: "var(--color-text-secondary)" }}>
                    <summary>Reasoning</summary>
                    <pre>{p.content}</pre>
                  </details>
                );
              }
              return null;
            })}
          </div>
        ))}
        {sending && <div className="loading">Generating response…</div>}
        {error && <div className="error-banner">{error}</div>}
      </div>
      {chatId && (
        <div className="composer">
          <div className="composer-row">
            <textarea
              className="textarea"
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
            <button className="btn primary" onClick={send} disabled={sending || !draft.trim()}>
              Send
            </button>
          </div>
        </div>
      )}
    </div>
  );
}