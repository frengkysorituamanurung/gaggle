import React, { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";

export default function HistoryScreen() {
  const { setScreen, setChatId, projectId, setProjectId } = useApp();
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
    setScreen("chat");
  };

  const rename = async (c: T.Chat) => {
    const title = prompt("New title", c.title);
    if (!title) return;
    const updated = await api.chats.update(c.id, { title } as any);
    setChats((prev) => prev.map((x) => (x.id === c.id ? updated : x)));
  };

  return (
    <div className="content">
      <h2>Session History</h2>
      <div style={{ marginBottom: 12 }}>
        <select className="select" style={{ width: 240 }} value={projectId || ""} onChange={(e) => setProjectId(e.target.value)}>
          {projects.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
        </select>
      </div>
      {loading && <div className="loading">Loading…</div>}
      {chats.length === 0 && !loading && <div className="empty-state">No chats yet.</div>}
      <table className="table">
        <thead><tr><th>Title</th><th>Mode</th><th>Permission</th><th>Updated</th><th></th></tr></thead>
        <tbody>
          {chats.map((c) => (
            <tr key={c.id}>
              <td>{c.title}</td>
              <td>{c.mode}</td>
              <td>{c.permission_mode}</td>
              <td>{new Date(c.updated_at).toLocaleString()}</td>
              <td>
                <button className="btn" onClick={() => open(c)}>Open</button>{" "}
                <button className="btn" onClick={() => rename(c)}>Rename</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}