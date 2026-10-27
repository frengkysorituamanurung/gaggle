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
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Extensions</h2>
      {loading && <div className="text-text-secondary text-sm">Loading…</div>}
      {error && <div className="text-danger text-sm">{error}</div>}
      {inv && (
        <>
          <p className="text-sm text-text-secondary">
            Active tools: {inv.active_tool_count} / recommended {inv.recommended_max_active_tools}
            {inv.exceeds_recommended_tool_limit && " (exceeds recommended)"}
          </p>
          <div className="grid gap-3 mt-3">
            {inv.extensions.map((e) => (
              <div key={e.id} className="bg-surface border border-border-primary rounded-xl p-4">
                <div className="flex items-center justify-between">
                  <div>
                    <span className="font-medium text-text-primary">{e.name}</span>
                    <span className="ml-2 text-xs px-2 py-0.5 rounded-full bg-surface-2 text-text-secondary">{e.kind}</span>
                    <div className="text-xs text-text-tertiary">{e.description}</div>
                  </div>
                  <button
                    className="px-3 py-1 rounded-md border border-border-primary bg-background-primary hover:bg-surface-2 text-text-primary text-sm"
                    onClick={async () => {
                      await api.extensions.setEnabled(e.id, !e.enabled);
                      refresh();
                    }}
                  >
                    {e.enabled ? "Disable" : "Enable"}
                  </button>
                </div>
                <div className="mt-2 text-sm text-text-secondary">Tools: {e.tool_count}</div>
                {e.tools.length > 0 && (
                  <ul className="mt-1 pl-5 text-xs text-text-secondary">
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