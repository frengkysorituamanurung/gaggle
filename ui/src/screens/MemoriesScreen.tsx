import React, { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";

export default function MemoriesScreen() {
  const { projectId } = useApp();
  const [memories, setMemories] = useState<T.Memory[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [kind, setKind] = useState("note");

  const refresh = () => {
    setLoading(true);
    api.memories.list(projectId || undefined).then(setMemories).catch((e) => setError(String(e))).finally(() => setLoading(false));
  };
  useEffect(refresh, [projectId]);

  const create = async () => {
    if (!projectId || !title.trim() || !content.trim()) return;
    try {
      await api.memories.create({ project_id: projectId, kind, title, content });
      setTitle(""); setContent("");
      refresh();
    } catch (e) { setError(String(e)); }
  };

  const remove = async (id: string) => {
    try {
      await api.memories.delete(id);
      refresh();
    } catch (e) { setError(String(e)); }
  };

  const exportMem = async () => {
    try {
      const blob = await api.memories.export();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a"); a.href = url; a.download = "memories.json"; a.click();
      URL.revokeObjectURL(url);
    } catch (e) { setError(String(e)); }
  };

  return (
    <div className="p-6 h-full overflow-y-auto">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold text-text-primary">Memories</h2>
        <Button size="sm" variant="secondary" onClick={exportMem}>Export</Button>
      </div>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
      {loading && <div className="text-text-secondary text-sm">Loading…</div>}
      {!projectId && <div className="text-text-secondary text-sm">Select a project first.</div>}
      {projectId && (
        <Card className="mb-4">
          <CardHeader><CardTitle className="text-text-primary text-base">New Memory</CardTitle></CardHeader>
          <CardContent>
            <div className="flex flex-col gap-2">
              <Input placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)} />
              <select className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm" value={kind} onChange={(e) => setKind(e.target.value)}>
                <option value="note">note</option>
                <option value="fact">fact</option>
                <option value="preference">preference</option>
              </select>
              <textarea className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm min-h-[60px]" placeholder="Content" value={content} onChange={(e) => setContent(e.target.value)} />
              <Button variant="default" size="sm" onClick={create}>Create Memory</Button>
            </div>
          </CardContent>
        </Card>
      )}
      <div className="grid gap-3">
        {memories.map((m) => (
          <Card key={m.id}>
            <CardHeader>
              <CardTitle className="text-text-primary text-base">{m.title}</CardTitle>
              <div className="text-xs text-text-tertiary">{m.kind} · {m.scope} · {new Date(m.updated_at).toLocaleString()}</div>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-text-secondary whitespace-pre-wrap">{m.content}</p>
              {m.tags && <div className="mt-2 text-xs text-text-tertiary">tags: {m.tags}</div>}
              <div className="mt-2">
                <Button size="sm" variant="destructive" onClick={() => remove(m.id)}>Delete</Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
      {!loading && memories.length === 0 && <div className="text-text-secondary text-center py-12">No memories yet.</div>}
    </div>
  );
}