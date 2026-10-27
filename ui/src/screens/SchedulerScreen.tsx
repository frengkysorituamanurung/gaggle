import React, { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";

export default function SchedulerScreen() {
  const { projectId } = useApp();
  const [schedules, setSchedules] = useState<T.Schedule[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setLoading(true);
    api.schedules(projectId || undefined).then(setSchedules).catch((e) => setError(String(e))).finally(() => setLoading(false));
  }, [projectId]);
  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text mb-4">Scheduler</h2>
      {loading && <div className="text-text-secondary text-sm">Loading…</div>}
      {error && <div className="text-danger text-sm">{error}</div>}
      {schedules.length === 0 && !loading && <div className="text-text-secondary text-center py-12">No scheduled jobs.</div>}
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border text-text-secondary">
              <th className="text-left py-2 px-3">Recipe</th>
              <th className="text-left py-2 px-3">Cron</th>
              <th className="text-left py-2 px-3">Timezone</th>
              <th className="text-left py-2 px-3">Enabled</th>
              <th className="text-left py-2 px-3">Next run</th>
            </tr>
          </thead>
          <tbody>
            {schedules.map((s) => (
              <tr key={s.id} className="border-b border-border">
                <td className="py-2 px-3 text-text">{s.recipe_id}</td>
                <td className="py-2 px-3"><code className="text-xs">{s.cron_expr}</code></td>
                <td className="py-2 px-3 text-text-secondary">{s.timezone}</td>
                <td className="py-2 px-3 text-text-secondary">{s.enabled ? "yes" : "no"}</td>
                <td className="py-2 px-3 text-text-secondary">{s.next_run_at ? new Date(s.next_run_at).toLocaleString() : "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}