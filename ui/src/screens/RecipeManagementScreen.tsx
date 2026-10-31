import React, { useEffect, useState } from "react";
import { api } from "../lib/api";
import type * as T from "../types";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";

export default function RecipeManagementScreen() {
  const [recipes, setRecipes] = useState<T.Recipe[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [newName, setNewName] = useState("");
  const [newTitle, setNewTitle] = useState("");
  const [newDescription, setNewDescription] = useState("");

  const load = async () => {
    setLoading(true);
    try {
      const rs = await api.recipes.list();
      setRecipes(rs);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const create = async () => {
    if (!newName.trim() || !newTitle.trim()) return;
    try {
      await api.recipes.create({
        name: newName.trim(),
        title: newTitle.trim(),
        description: newDescription.trim(),
      });
      setNewName(""); setNewTitle(""); setNewDescription(""); setShowForm(false);
      await load();
    } catch (e) { setError(String(e)); }
  };

  const remove = async (id: string) => {
    if (!confirm("Delete this recipe?")) return;
    try {
      await api.recipes.delete(id);
      await load();
    } catch (e) { setError(String(e)); }
  };

  const execute = async (id: string) => {
    try {
      await api.recipes.execute(id);
      await load();
    } catch (e) { setError(String(e)); }
  };

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Recipe Management</h2>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
      <Button size="sm" onClick={() => setShowForm(!showForm)} className="mb-4">
        {showForm ? "Cancel" : "Create Recipe"}
      </Button>

      {showForm && (
        <Card className="mb-4">
          <CardContent className="p-4 flex flex-col gap-2">
            <Input placeholder="Name" value={newName} onChange={(e) => setNewName(e.target.value)} />
            <Input placeholder="Title" value={newTitle} onChange={(e) => setNewTitle(e.target.value)} />
            <Input placeholder="Description" value={newDescription} onChange={(e) => setNewDescription(e.target.value)} />
            <Button size="sm" onClick={create} disabled={!newName.trim() || !newTitle.trim()}>Save</Button>
          </CardContent>
        </Card>
      )}

      {loading && <div className="text-text-secondary text-sm">Loading...</div>}

      {recipes.map((r) => (
        <Card key={r.id} className="mb-2">
          <CardContent className="p-4">
            <div className="flex items-center justify-between mb-1">
              <span className="font-medium text-text-primary">{r.title || r.name}</span>
              <span className="text-xs text-text-tertiary">{r.name}</span>
            </div>
            {r.description && <p className="text-sm text-text-secondary mb-2">{r.description}</p>}
            <div className="flex gap-2">
              <Button size="sm" onClick={() => execute(r.id)}>Execute</Button>
              <Button size="sm" variant="secondary" onClick={() => remove(r.id)}>Delete</Button>
            </div>
          </CardContent>
        </Card>
      ))}
      {recipes.length === 0 && !loading && (
        <div className="text-text-secondary text-sm">No recipes yet.</div>
      )}
    </div>
  );
}