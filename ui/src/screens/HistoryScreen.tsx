import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";

export default function HistoryScreen() {
  const navigate = useNavigate();
  const { setChatId, projectId, setProjectId } = useApp();
  const [projects, setProjects] = useState<T.ProjectStatus[]>([]);
  const [chats, setChats] = useState<T.Chat[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    api.projects.list().then((ps) => {
      setProjects(ps);
      if (!projectId && ps.length) setProjectId(ps[0].id);
    }).catch(() => {});
  }, []);

  useEffect(() => {
    if (!projectId) return;
    setLoading(true);
    api.projectChats(projectId).then(setChats).finally(() => setLoading(false));
  }, [projectId]);

  const open = (c: T.Chat) => {
    setChatId(c.id);
    navigate("/chat");
  };

  const rename = async (c: T.Chat) => {
    const title = prompt("New title", c.title);
    if (!title) return;
    const updated = await api.chats.update(c.id, { title } as any);
    setChats((prev) => prev.map((x) => (x.id === c.id ? updated : x)));
  };

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Session History</h2>
      <div className="mb-4">
        <select className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm" value={projectId || ""} onChange={(e) => setProjectId(e.target.value)}>
          {projects.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
        </select>
      </div>
      {loading && <div className="text-text-secondary text-sm">Loading…</div>}
      {chats.length === 0 && !loading && <div className="text-text-secondary text-center py-12">No chats yet.</div>}
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border-primary text-text-secondary">
              <th className="text-left py-2 px-3">Title</th>
              <th className="text-left py-2 px-3">Mode</th>
              <th className="text-left py-2 px-3">Permission</th>
              <th className="text-left py-2 px-3">Updated</th>
              <th className="text-left py-2 px-3"></th>
            </tr>
          </thead>
          <tbody>
            {chats.map((c) => (
              <tr key={c.id} className="border-b border-border-primary">
                <td className="py-2 px-3 text-text-primary">{c.title}</td>
                <td className="py-2 px-3 text-text-secondary">{c.mode}</td>
                <td className="py-2 px-3 text-text-secondary">{c.permission_mode}</td>
                <td className="py-2 px-3 text-text-secondary">{new Date(c.updated_at).toLocaleString()}</td>
                <td className="py-2 px-3">
                  <button className="px-2 py-1 rounded-md border border-border-primary bg-background-primary hover:bg-surface-2 text-text-primary text-xs mr-1" onClick={() => open(c)}>Open</button>
                  <button className="px-2 py-1 rounded-md border border-border-primary bg-background-primary hover:bg-surface-2 text-text-primary text-xs" onClick={() => rename(c)}>Rename</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}