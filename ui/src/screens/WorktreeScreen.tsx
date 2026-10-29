import React, { useEffect, useState } from "react";
import { api } from "../lib/api";
import type * as T from "../types";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";
import { Button } from "../components/ui/Button";

interface WorktreeScreenProps {
  projectId: string;
}

export default function WorktreeScreen({ projectId }: WorktreeScreenProps) {
  const [worktrees, setWorktrees] = useState<T.Worktree[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [diff, setDiff] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    if (!projectId) return;
    try {
      const wts = await api.worktrees.list(projectId);
      setWorktrees(wts);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    load();
  }, [projectId]);

  const viewDiff = async (id: string) => {
    setSelected(id);
    setLoading(true);
    try {
      const d = await api.worktrees.diff(id);
      setDiff(d);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const merge = async (id: string) => {
    try {
      await api.worktrees.merge(id);
      await load();
      setSelected(null);
      setDiff("");
    } catch (e) {
      setError(String(e));
    }
  };

  const discard = async (id: string) => {
    if (!confirm("Discard all changes in this worktree?")) return;
    try {
      await api.worktrees.discard(id);
      await load();
      setSelected(null);
      setDiff("");
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Worktrees</h2>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
      {worktrees.length === 0 && (
        <div className="text-text-secondary text-sm">No worktrees for this project.</div>
      )}
      <div className="flex gap-4">
        <div className="w-1/3">
          {worktrees.map((wt) => (
            <Card key={wt.id} className="mb-2 cursor-pointer" onClick={() => viewDiff(wt.id)}>
              <CardContent className="p-3">
                <div className="font-medium text-text-primary text-sm">{wt.branch}</div>
                <div className="text-xs text-text-tertiary">{wt.status}</div>
              </CardContent>
            </Card>
          ))}
        </div>
        <div className="flex-1">
          {selected && (
            <div>
              <div className="flex gap-2 mb-2">
                <Button size="sm" onClick={() => merge(selected)}>Merge</Button>
                <Button size="sm" variant="secondary" onClick={() => discard(selected)}>Discard</Button>
              </div>
              {loading && <div className="text-text-secondary text-sm">Loading diff...</div>}
              {!loading && diff && (
                <pre className="text-xs text-text-secondary whitespace-pre-wrap overflow-auto max-h-[60vh] bg-surface-1 p-3 rounded">
                  {diff}
                </pre>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}