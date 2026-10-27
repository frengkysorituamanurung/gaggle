import React, { useEffect, useState } from "react";
import { api } from "../lib/api";
import type * as T from "../types";

export default function ExtensionsScreen() {
  const [inv, setInv] = useState<T.ExtensionsInventory | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const refresh = () => {
    setLoading(true);
    api.extensions.inventory().then(setInv).catch((e) => setError(String(e))).finally(() => setLoading(false));
  };
  useEffect(refresh, []);
  return (
    <div className="content">
      <h2>Extensions</h2>
      {loading && <div className="loading">Loading…</div>}
      {error && <div className="error-banner">{error}</div>}
      {inv && (
        <>
          <p style={{ color: "var(--color-text-secondary)", fontSize: "var(--text-sm)" }}>
            Active tools: {inv.active_tool_count} / recommended {inv.recommended_max_active_tools}
            {inv.exceeds_recommended_tool_limit && " (exceeds recommended)"}
          </p>
          <div style={{ display: "grid", gap: 12, marginTop: 12 }}>
            {inv.extensions.map((e) => (
              <div key={e.id} className="card">
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div>
                    <strong>{e.name}</strong>
                    <span className="badge" style={{ marginLeft: 8 }}>{e.kind}</span>
                    <div style={{ fontSize: "var(--text-xs)", color: "var(--color-text-muted)" }}>{e.description}</div>
                  </div>
                  <button
                    className="btn"
                    onClick={async () => {
                      await api.extensions.setEnabled(e.id, !e.enabled);
                      refresh();
                    }}
                  >
                    {e.enabled ? "Disable" : "Enable"}
                  </button>
                </div>
                <div style={{ marginTop: 8, fontSize: "var(--text-sm)", color: "var(--color-text-secondary)" }}>
                  Tools: {e.tool_count}
                </div>
                {e.tools.length > 0 && (
                  <ul style={{ margin: "4px 0 0", paddingLeft: 18, fontSize: "var(--text-xs)" }}>
                    {e.tools.map((t) => (
                      <li key={t.tool_name}>{t.tool_name} {t.is_read_only ? "(read-only)" : ""}</li>
                    ))}
                  </ul>
                )}
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}