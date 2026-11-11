import React, { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type * as T from "../../types";
import { EmptyState } from "../EmptyState";
import { LoadingState } from "../LoadingState";
import { ErrorState } from "../ErrorState";

export function SkillsView() {
  const [skills, setSkills] = useState<T.SkillSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.skills().then(setSkills).catch((e) => setError(String(e))).finally(() => setLoading(false));
  }, []);

  if (loading) return <LoadingState />;
  if (error) return <ErrorState message={error} />;
  if (skills.length === 0) return <EmptyState title="No skills" description="No skills found." />;

  return (
    <div className="grid gap-3">
      {skills.map((s) => (
        <div key={s.path} className="rounded-md border border-border-primary bg-background-secondary p-3">
          <div className="font-medium text-text-primary">{s.name}</div>
          <p className="text-sm text-text-secondary mt-1">{s.description}</p>
          <div className="text-xs text-text-tertiary mt-1">{s.path}</div>
        </div>
      ))}
    </div>
  );
}

export default SkillsView;