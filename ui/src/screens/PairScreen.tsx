import React, { useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import ChatScreen from "./ChatScreen";
import { Button } from "../components/ui/Button";

const MAX = 10;

export default function PairScreen() {
  const { projectId } = useApp();
  const [params] = useSearchParams();
  const resume = params.get("resumeSessionId");
  const [sessions, setSessions] = useState<string[]>(resume ? [resume] : []);
  const [active, setActive] = useState<string | null>(resume || null);

  const newSession = async () => {
    if (!projectId) return;
    try {
      const c = await api.chats.create({ project_id: projectId, title: "New Chat", mode: "local", permission_mode: "ask" });
      setSessions((prev) => {
        const next = [...prev, c.id];
        if (next.length > MAX) next.shift();
        return next;
      });
      setActive(c.id);
    } catch { /* ignore */ }
  };

  return (
    <div className="flex h-full min-h-0">
      <div className="w-56 border-r border-border-primary p-2 flex flex-col gap-1 overflow-y-auto">
        <Button size="sm" variant="default" onClick={newSession}>+ New Session</Button>
        {sessions.map((s) => (
          <button
            key={s}
            onClick={() => setActive(s)}
            className={`text-left px-2 py-1.5 rounded-md text-sm truncate ${active === s ? "bg-background-tertiary text-text-primary" : "text-text-secondary hover:bg-background-secondary"}`}
          >
            {s.slice(0, 8)}…
          </button>
        ))}
      </div>
      <div className="flex-1 min-w-0">
        {active ? <ChatScreen /> : <div className="flex items-center justify-center h-full text-text-secondary">Select or create a session.</div>}
      </div>
    </div>
  );
}