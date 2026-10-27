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
    <div className="content">
      <h2>Scheduler</h2>
      {loading && <div className="loading">Loading…</div>}
      {error && <div className="error-banner">{error}</div>}
      {schedules.length === 0 && !loading && <div className="empty-state">No scheduled jobs.</div>}
      <table className="table">
        <thead><tr><th>Recipe</th><th>Cron</th><th>Timezone</th><th>Enabled</th><th>Next run</th></tr></thead>
        <tbody>
          {schedules.map((s) => (
            <tr key={s.id}>
              <td>{s.recipe_id}</td>
              <td><code>{s.cron_expr}</code></td>
              <td>{s.timezone}</td>
              <td>{s.enabled ? "yes" : "no"}</td>
              <td>{s.next_run_at ? new Date(s.next_run_at).toLocaleString() : "—"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}