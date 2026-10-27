import React, { useEffect, useState } from "react";
import { api } from "../lib/api";
import type * as T from "../types";

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
      {loading && <div className="text-text-secondary text-sm">Loading…</div>}
      {error && <div className="text-danger text-sm">{error}</div>}
      {skills.length === 0 && !loading && <div className="text-text-secondary text-center py-12">No skills found.</div>}
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