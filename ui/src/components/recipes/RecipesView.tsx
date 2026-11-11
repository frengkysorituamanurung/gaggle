import React, { useEffect, useState } from "react";
import { useApp } from "../../lib/store";
import { api } from "../../lib/api";
import type * as T from "../../types";
import { Button } from "../ui/Button";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card";
import { EmptyState } from "../EmptyState";
import { LoadingState } from "../LoadingState";
import { ErrorState } from "../ErrorState";

export function RecipesView() {
  const { projectId } = useApp();
  const [recipes, setRecipes] = useState<T.Recipe[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = () => {
    setLoading(true);
    api.recipes().then(setRecipes).catch((e) => setError(String(e))).finally(() => setLoading(false));
  };
  useEffect(refresh, []);

  const remove = async (id: string) => {
    try {
      await api.recipe.delete(id);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const execute = async (r: T.Recipe) => {
    if (!projectId) {
      setError("Select a project first.");
      return;
    }
    try {
      const outcome = await api.recipe.execute(r.id, { project_id: projectId });
      alert(`Recipe executed: ${outcome.outcome}`);
    } catch (e) {
      setError(String(e));
    }
  };

  if (loading) return <LoadingState />;
  if (error) return <ErrorState message={error} onRetry={refresh} />;
  if (recipes.length === 0) return <EmptyState title="No recipes" description="Create or import a recipe to get started." />;

  return (
    <div className="grid gap-3">
      {recipes.map((r) => (
        <Card key={r.id}>
          <CardHeader>
            <CardTitle className="text-text-primary text-base">{r.title}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-xs text-text-tertiary mb-2">{r.name}</div>
            {r.description && <p className="text-sm text-text-secondary mb-2">{r.description}</p>}
            <div className="flex gap-2">
              <Button size="sm" variant="default" onClick={() => execute(r)} disabled={!projectId}>Run</Button>
              <Button size="sm" variant="destructive" onClick={() => remove(r.id)}>Delete</Button>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

export default RecipesView;