import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";
import { Button } from "../components/ui/Button";
import { AutoTextarea } from "../components/ui/AutoTextarea";
import { Card, CardContent } from "../components/ui/Card";

export default function HubScreen() {
  const navigate = useNavigate();
  const { projectId, setProjectId, setChatId } = useApp();
  const [projects, setProjects] = useState<T.ProjectStatus[]>([]);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.projects.list().then((ps) => {
      setProjects(ps);
      if (!projectId && ps.length) setProjectId(ps[0].id);
    }).catch(() => {});
  }, []);

  const hour = new Date().getHours();
  const greeting = hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";

  const start = async () => {
    if (!projectId || !draft.trim()) { navigate("/chat"); return; }
    setSending(true);
    try {
      const chat = await api.chats.create({ project_id: projectId, title: draft.slice(0, 40) || "New Chat", mode: "local", permission_mode: "ask" });
      setChatId(chat.id);
      await api.chats.submit(chat.id, draft, "local-mock");
      navigate("/chat");
    } catch (e) {
      setError(String(e));
    } finally {
      setSending(false);
    }
  };

  return (
    <div className="flex flex-col items-center justify-center h-full gap-6 p-6">
      <div className="text-center">
        <h1 className="text-3xl font-semibold text-text-primary">{greeting}</h1>
        <p className="text-text-secondary mt-1">What would you like to work on?</p>
      </div>
      {error && <div className="text-text-danger text-sm">{error}</div>}
      {projects.length === 0 ? (
        <Card className="w-full max-w-md">
          <CardContent className="pt-6">
            <p className="text-text-secondary text-sm mb-3">Register a project to start.</p>
            <Button variant="default" onClick={() => navigate("/chat")}>Go to Chat</Button>
          </CardContent>
        </Card>
      ) : (
        <Card className="w-full max-w-2xl">
          <CardContent className="pt-6">
            <AutoTextarea
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              placeholder="Message Gaggle… (Enter to send)"
              minRows={2}
              maxRows={8}
              onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); start(); } }}
            />
            <div className="flex items-center justify-between mt-3">
              <select className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm" value={projectId || ""} onChange={(e) => setProjectId(e.target.value)}>
                {projects.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
              </select>
              <Button variant="default" onClick={start} disabled={sending || !draft.trim()}>
                {sending ? "Sending…" : "Send"}
              </Button>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}