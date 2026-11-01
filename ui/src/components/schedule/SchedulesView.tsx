import React, { useEffect, useState } from "react";
import { useApp } from "../../lib/store";
import { api } from "../../lib/api";
import type * as T from "../../types";
import { Button } from "../ui/Button";
import { EmptyState } from "../EmptyState";
import { LoadingState } from "../LoadingState";
import { ErrorState } from "../ErrorState";
import { ScheduleModal } from "./ScheduleModal";

export function SchedulesView() {
  const { projectId } = useApp();
  const [schedules, setSchedules] = useState<T.Schedule[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [modalOpen, setModalOpen] = useState(false);

  const refresh = () => {
    setLoading(true);
    api.schedules(projectId || undefined).then(setSchedules).catch((e) => setError(String(e))).finally(() => setLoading(false));
  };
  useEffect(refresh, [projectId]);

  const remove = async (id: string) => {
    try {
      await api.schedule.delete(id);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  if (loading) return <LoadingState />;
  if (error) return <ErrorState message={error} onRetry={refresh} />;
  if (schedules.length === 0) return (
    <>
      <EmptyState title="No schedules" description="Create a schedule to run recipes automatically." />
      <div className="mt-2">
        <Button size="sm" variant="default" onClick={() => setModalOpen(true)}>Create Schedule</Button>
      </div>
      <ScheduleModal open={modalOpen} onClose={() => setModalOpen(false)} onSaved={refresh} projectId={projectId} />
    </>
  );

  return (
    <div className="space-y-3">
      <div className="flex justify-end">
        <Button size="sm" variant="default" onClick={() => setModalOpen(true)}>Create Schedule</Button>
      </div>
      {schedules.map((s) => (
        <div key={s.id} className="rounded-md border border-border-primary bg-background-secondary p-3 text-sm">
          <div className="font-medium text-text-primary">{s.recipe_id}</div>
          <div className="text-xs text-text-secondary">Cron: {s.cron_expr} · {s.timezone}</div>
          <div className="text-xs text-text-tertiary">Enabled: {s.enabled ? "yes" : "no"}</div>
          <div className="mt-2">
            <Button size="sm" variant="destructive" onClick={() => remove(s.id)}>Delete</Button>
          </div>
        </div>
      ))}
      <ScheduleModal open={modalOpen} onClose={() => setModalOpen(false)} onSaved={refresh} projectId={projectId} />
    </div>
  );
}

export default SchedulesView;