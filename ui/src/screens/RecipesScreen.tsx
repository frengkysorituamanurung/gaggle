import React, { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";

export default function RecipesScreen() {
  const { projectId } = useApp();
  const [recipes, setRecipes] = useState<T.Recipe[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    api.recipes().then(setRecipes).catch((e) => setError(String(e))).finally(() => setLoading(false));
  }, []);
  return (
    <div className="content">
      <h2>Recipes</h2>
      {loading && <div className="loading">Loading…</div>}
      {error && <div className="error-banner">{error}</div>}
      {recipes.length === 0 && !loading && <div className="empty-state">No recipes available.</div>}
      <div style={{ display: "grid", gap: 12, marginTop: 12 }}>
        {recipes.map((r) => (
          <div key={r.id} className="card">
            <strong>{r.title}</strong>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--color-text-muted)" }}>{r.name}</div>
            {r.description && <p style={{ margin: "4px 0 0", color: "var(--color-text-secondary)", fontSize: "var(--text-sm)" }}>{r.description}</p>}
            {r.prompt && <pre style={{ marginTop: 8 }}>{r.prompt}</pre>}
          </div>
        ))}
      </div>
      {!projectId && <p className="empty-state">Select a project in Chat to run recipes.</p>}
    </div>
  );
}