import React, { useEffect, useState } from "react";
import { api } from "../lib/api";
import type * as T from "../types";
import { EmptyState } from "../components/EmptyState";
import { ErrorState } from "../components/ErrorState";
import { LoadingState } from "../components/LoadingState";

export default function SkillsScreen() {
  const [skills, setSkills] = useState<T.SkillSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    api.skills().then(setSkills).catch((e) => setError(String(e))).finally(() => setLoading(false));
  }, []);
  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Skills</h2>
      {loading && <LoadingState />}
      {error && <ErrorState message={error} />}
      {skills.length === 0 && !loading && !error && (
        <EmptyState title="No skills found." description="Install or discover skills to get started." />
      )}
      <div className="grid gap-3 mt-3">
        {skills.map((s) => (
          <div key={s.path} className="bg-surface border border-border-primary rounded-xl p-4">
            <div className="font-medium text-text-primary">{s.name}</div>
            <p className="mt-1 text-sm text-text-secondary">{s.description}</p>
            <div className="mt-2 text-xs text-text-tertiary">{s.path}</div>
          </div>
        ))}
      </div>
    </div>
  );
}