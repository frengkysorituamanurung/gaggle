import React, { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";

export default function SchedulerScreen() {
  const { projectId } = useApp();
  const [schedules, setSchedules] = useState<T.Schedule[]>([]);
  const [recipes, setRecipes] = useState<T.Recipe[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [recipeId, setRecipeId] = useState("");
  const [cronExpr, setCronExpr] = useState("0 * * * *");
  const [timezone, setTimezone] = useState("UTC");
  const [enabled, setEnabled] = useState(true);

  const refresh = () => {
    setLoading(true);
    Promise.all([api.schedules(projectId || undefined), api.recipes()])
      .then(([s, r]) => { setSchedules(s); setRecipes(r); if (!recipeId && r.length) setRecipeId(r[0].id); })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  };
  useEffect(refresh, [projectId]);

  const create = async () => {
    if (!recipeId || !cronExpr.trim()) return;
    try {
      await api.schedule.create({
        recipe_id: recipeId,
        project_id: projectId || null,
        cron_expr: cronExpr.trim(),
        timezone: timezone.trim() || "UTC",
        enabled: enabled ? 1 : 0,
      });
      setCronExpr("0 * * * *");
      refresh();
    } catch (e) { setError(String(e)); }
  };

  const toggle = async (s: T.Schedule) => {
    try { await api.schedule.update(s.id, { enabled: s.enabled ? 0 : 1 }); refresh(); } catch (e) { setError(String(e)); }
  };

  const remove = async (id: string) => {
    try { await api.schedule.delete(id); refresh(); } catch (e) { setError(String(e)); }
  };

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Scheduler</h2>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}

      <Card className="mb-4">
        <CardHeader><CardTitle className="text-text-primary text-base">Create Schedule</CardTitle></CardHeader>
        <CardContent>
          <div className="flex flex-col gap-2">
            <select className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm" value={recipeId} onChange={(e) => setRecipeId(e.target.value)}>
              <option value="">Select recipe</option>
              {recipes.map((r) => <option key={r.id} value={r.id}>{r.title}</option>)}
            </select>
            <Input placeholder="Cron expression" value={cronExpr} onChange={(e) => setCronExpr(e.target.value)} />
            <Input placeholder="Timezone" value={timezone} onChange={(e) => setTimezone(e.target.value)} />
            <label className="flex items-center gap-2 text-sm text-text-primary">
              <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} /> Enabled
            </label>
            <Button variant="default" size="sm" onClick={create} disabled={!recipeId || !cronExpr.trim()}>Create</Button>
          </div>
        </CardContent>
      </Card>

      {loading && <div className="text-text-secondary text-sm">Loading…</div>}
      {schedules.length === 0 && !loading && <div className="text-text-secondary text-center py-12">No scheduled jobs.</div>}
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border-primary text-text-secondary">
              <th className="text-left py-2 px-3">Recipe</th>
              <th className="text-left py-2 px-3">Cron</th>
              <th className="text-left py-2 px-3">Timezone</th>
              <th className="text-left py-2 px-3">Enabled</th>
              <th className="text-left py-2 px-3">Next run</th>
              <th className="text-left py-2 px-3">Actions</th>
            </tr>
          </thead>
          <tbody>
            {schedules.map((s) => (
              <tr key={s.id} className="border-b border-border-primary">
                <td className="py-2 px-3 text-text-primary">{s.recipe_id}</td>
                <td className="py-2 px-3"><code className="text-xs">{s.cron_expr}</code></td>
                <td className="py-2 px-3 text-text-secondary">{s.timezone}</td>
                <td className="py-2 px-3 text-text-secondary">{s.enabled ? "yes" : "no"}</td>
                <td className="py-2 px-3 text-text-secondary">{s.next_run_at ? new Date(s.next_run_at).toLocaleString() : "—"}</td>
                <td className="py-2 px-3">
                  <div className="flex gap-2">
                    <Button size="sm" variant="secondary" onClick={() => toggle(s)}>{s.enabled ? "Disable" : "Enable"}</Button>
                    <Button size="sm" variant="destructive" onClick={() => remove(s.id)}>Delete</Button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
