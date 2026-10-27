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
    <div className="content">
      <h2>Skills</h2>
      {loading && <div className="loading">Loading…</div>}
      {error && <div className="error-banner">{error}</div>}
      {skills.length === 0 && !loading && <div className="empty-state">No skills found.</div>}
      <div style={{ display: "grid", gap: 12, marginTop: 12 }}>
        {skills.map((s) => (
          <div key={s.path} className="card">
            <strong>{s.name}</strong>
            <p style={{ margin: "4px 0 0", color: "var(--color-text-secondary)", fontSize: "var(--text-sm)" }}>{s.description}</p>
            <div style={{ marginTop: 4, fontSize: "var(--text-xs)", color: "var(--color-text-muted)" }}>{s.path}</div>
          </div>
        ))}
      </div>
    </div>
  );
}