import React, { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";

export default function RecipesScreen() {
  const { projectId } = useApp();
  const [recipes, setRecipes] = useState<T.Recipe[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [title, setTitle] = useState("");
  const [prompt, setPrompt] = useState("");
  const [description, setDescription] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [executing, setExecuting] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);

  const refresh = () => {
    setLoading(true);
    api.recipes().then(setRecipes).catch((e) => setError(String(e))).finally(() => setLoading(false));
  };
  useEffect(refresh, []);

  const resetForm = () => {
    setName(""); setTitle(""); setPrompt(""); setDescription(""); setEditingId(null);
  };

  const save = async () => {
    if (!name.trim() || !title.trim()) return;
    try {
      if (editingId) {
        await api.recipe.update(editingId, { name, title, prompt: prompt || null, description: description || null });
      } else {
        await api.recipe.create({ name, title, prompt: prompt || null, description: description || null });
      }
      resetForm();
      refresh();
    } catch (e) { setError(String(e)); }
  };

  const edit = (r: T.Recipe) => {
    setEditingId(r.id);
    setName(r.name);
    setTitle(r.title);
    setPrompt(r.prompt || "");
    setDescription(r.description || "");
  };

  const remove = async (id: string) => {
    try { await api.recipe.delete(id); refresh(); } catch (e) { setError(String(e)); }
  };

  const execute = async (r: T.Recipe) => {
    if (!projectId) { setError("Select a project first."); return; }
    setExecuting(r.id);
    setResult(null);
    try {
      const outcome = await api.recipe.execute(r.id, { project_id: projectId, mode: "local", permission_mode: "ask" });
      setResult(`Recipe executed: ${outcome.outcome}`);
    } catch (e) { setError(String(e)); }
    finally { setExecuting(null); }
  };

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Recipes</h2>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
      {result && <div className="text-text-success text-sm mb-2">{result}</div>}

      <Card className="mb-4">
        <CardHeader><CardTitle className="text-text-primary text-base">{editingId ? "Edit Recipe" : "Create Recipe"}</CardTitle></CardHeader>
        <CardContent>
          <div className="flex flex-col gap-2">
            <Input placeholder="Name" value={name} onChange={(e) => setName(e.target.value)} />
            <Input placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)} />
            <Input placeholder="Description" value={description} onChange={(e) => setDescription(e.target.value)} />
            <textarea className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm min-h-[80px]" placeholder="Prompt" value={prompt} onChange={(e) => setPrompt(e.target.value)} />
            <div className="flex gap-2">
              <Button variant="default" size="sm" onClick={save} disabled={!name.trim() || !title.trim()}>{editingId ? "Update" : "Create"}</Button>
              {editingId && <Button variant="secondary" size="sm" onClick={resetForm}>Cancel</Button>}
            </div>
          </div>
        </CardContent>
      </Card>

      {loading && <div className="text-text-secondary text-sm">Loading…</div>}
      {recipes.length === 0 && !loading && <div className="text-text-secondary text-center py-12">No recipes available.</div>}
      <div className="grid gap-3 mt-3">
        {recipes.map((r) => (
          <div key={r.id} className="bg-surface border border-border-primary rounded-xl p-4">
            <div className="flex items-center justify-between">
              <div>
                <div className="font-medium text-text-primary">{r.title}</div>
                <div className="text-xs text-text-tertiary">{r.name}</div>
              </div>
              <div className="flex gap-2">
                <Button size="sm" variant="secondary" onClick={() => execute(r)} disabled={executing === r.id || !projectId}>
                  {executing === r.id ? "Running…" : "Execute"}
                </Button>
                <Button size="sm" variant="secondary" onClick={() => edit(r)}>Edit</Button>
                <Button size="sm" variant="destructive" onClick={() => remove(r.id)}>Delete</Button>
              </div>
            </div>
            {r.description && <p className="mt-1 text-sm text-text-secondary">{r.description}</p>}
            {r.prompt && <pre className="mt-2 text-xs bg-code-bg p-2 rounded-md overflow-x-auto">{r.prompt}</pre>}
          </div>
        ))}
      </div>
    </div>
  );
}
