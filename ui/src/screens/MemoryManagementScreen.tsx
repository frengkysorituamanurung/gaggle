import React, { useEffect, useState } from "react";
import { api } from "../lib/api";
import type * as T from "../types";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";

export default function MemoryManagementScreen() {
  const [memories, setMemories] = useState<T.Memory[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  const [newTitle, setNewTitle] = useState("");
  const [newContent, setNewContent] = useState("");
  const [newKind, setNewKind] = useState("fact");
  const [showForm, setShowForm] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      const ms = await api.memories.list();
      setMemories(ms);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const create = async () => {
    if (!newTitle.trim() || !newContent.trim()) return;
    try {
      await api.memories.create({ title: newTitle.trim(), content: newContent.trim(), kind: newKind });
      setNewTitle(""); setNewContent(""); setShowForm(false);
      await load();
    } catch (e) { setError(String(e)); }
  };

  const remove = async (id: string) => {
    if (!confirm("Delete this memory?")) return;
    try {
      await api.memories.delete(id);
      await load();
    } catch (e) { setError(String(e)); }
  };

  const update = async (id: string) => {
    try {
      await api.memories.update(id, { title: newTitle.trim(), content: newContent.trim() });
      setEditing(null);
      setNewTitle(""); setNewContent("");
      await load();
    } catch (e) { setError(String(e)); }
  };

  const startEdit = (m: T.Memory) => {
    setEditing(m.id);
    setNewTitle(m.title);
    setNewContent(m.content);
  };

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Memory Management</h2>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
      <Button size="sm" onClick={() => setShowForm(!showForm)} className="mb-4">
        {showForm ? "Cancel" : "Add Memory"}
      </Button>

      {showForm && (
        <Card className="mb-4">
          <CardContent className="p-4 flex flex-col gap-2">
            <Input placeholder="Title" value={newTitle} onChange={(e) => setNewTitle(e.target.value)} />
            <select
              className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
              value={newKind}
              onChange={(e) => setNewKind(e.target.value)}
            >
              <option value="fact">Fact</option>
              <option value="preference">Preference</option>
              <option value="instruction">Instruction</option>
            </select>
            <textarea
              className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm min-h-[80px]"
              placeholder="Content"
              value={newContent}
              onChange={(e) => setNewContent(e.target.value)}
            />
            <Button size="sm" onClick={create}>Save</Button>
          </CardContent>
        </Card>
      )}

      {loading && <div className="text-text-secondary text-sm">Loading...</div>}

      {memories.map((m) => (
        <Card key={m.id} className="mb-2">
          <CardContent className="p-4">
            {editing === m.id ? (
              <div className="flex flex-col gap-2">
                <Input value={newTitle} onChange={(e) => setNewTitle(e.target.value)} />
                <textarea
                  className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm min-h-[80px]"
                  value={newContent}
                  onChange={(e) => setNewContent(e.target.value)}
                />
                <div className="flex gap-2">
                  <Button size="sm" onClick={() => update(m.id)}>Save</Button>
                  <Button size="sm" variant="secondary" onClick={() => { setEditing(null); setNewTitle(""); setNewContent(""); }}>Cancel</Button>
                </div>
              </div>
            ) : (
              <div>
                <div className="flex items-center justify-between mb-1">
                  <span className="font-medium text-text-primary">{m.title}</span>
                  <span className="text-xs px-2 py-0.5 rounded-full bg-surface-2 text-text-secondary">{m.kind}</span>
                </div>
                <p className="text-sm text-text-secondary mb-2">{m.content}</p>
                <div className="flex gap-2">
                  <Button size="sm" variant="secondary" onClick={() => startEdit(m)}>Edit</Button>
                  <Button size="sm" variant="secondary" onClick={() => remove(m.id)}>Delete</Button>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      ))}
      {memories.length === 0 && !loading && (
        <div className="text-text-secondary text-sm">No memories yet.</div>
      )}
    </div>
  );
}