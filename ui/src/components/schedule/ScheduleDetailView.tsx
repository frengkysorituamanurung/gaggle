import React from "react";
import type * as T from "../../types";

export function ScheduleDetailView({ schedule }: { schedule: T.Schedule | null }) {
  if (!schedule) return <div className="text-sm text-text-secondary">No schedule selected.</div>;
  return (
    <div className="space-y-2 text-sm">
      <div className="font-medium text-text-primary">{schedule.recipe_id}</div>
      <div className="text-text-secondary">Cron: {schedule.cron_expr}</div>
      <div className="text-text-secondary">Timezone: {schedule.timezone}</div>
      <div className="text-text-tertiary">Next run: {schedule.next_run_at ? new Date(schedule.next_run_at).toLocaleString() : "—"}</div>
      <div className="text-text-tertiary">Last run: {schedule.last_run_at ? new Date(schedule.last_run_at).toLocaleString() : "—"}</div>
    </div>
  );
}

export default ScheduleDetailView;