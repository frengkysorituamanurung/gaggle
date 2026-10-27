import React, { useEffect, useState } from "react";
import { api } from "../lib/api";
import type * as T from "../types";

export default function RecipesScreen() {
  const [recipes, setRecipes] = useState<T.Recipe[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    api.recipes().then(setRecipes).catch((e) => setError(String(e))).finally(() => setLoading(false));
  }, []);
  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Recipes</h2>
      {loading && <div className="text-text-secondary text-sm">Loading…</div>}
      {error && <div className="text-danger text-sm">{error}</div>}
      {recipes.length === 0 && !loading && <div className="text-text-secondary text-center py-12">No recipes available.</div>}
      <div className="grid gap-3 mt-3">
        {recipes.map((r) => (
          <div key={r.id} className="bg-surface border border-border-primary rounded-xl p-4">
            <div className="font-medium text-text-primary">{r.title}</div>
            <div className="text-xs text-text-tertiary">{r.name}</div>
            {r.description && <p className="mt-1 text-sm text-text-secondary">{r.description}</p>}
            {r.prompt && <pre className="mt-2 text-xs bg-code-bg p-2 rounded-md overflow-x-auto">{r.prompt}</pre>}
          </div>
        ))}
      </div>
    </div>
  );
}