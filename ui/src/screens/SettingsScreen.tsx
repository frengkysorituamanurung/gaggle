import React, { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";

export default function SettingsScreen() {
  const { theme, setTheme } = useApp();
  const [providers, setProviders] = useState<T.ProviderStatus[]>([]);
  const [settings, setSettings] = useState<T.AppSetting[]>([]);
  const [approvals, setApprovals] = useState<T.Approval[]>([]);
  const [error, setError] = useState<string | null>(null);

  // provider form
  const [pName, setPName] = useState("");
  const [pKind, setPKind] = useState("openai_compat");
  const [pAuth, setPAuth] = useState("api_key");
  const [pKey, setPKey] = useState("");
  const [pUrl, setPUrl] = useState("");

  const refresh = () => {
    api.providers.list().then(setProviders).catch(() => {});
    api.settings.list().then(setSettings).catch(() => {});
    api.approvals.pending().then(setApprovals).catch(() => {});
  };
  useEffect(refresh, []);

  const createProvider = async () => {
    try {
      await api.providers.create({
        kind: pKind,
        name: pName || pKind,
        base_url: pUrl || undefined,
        auth_kind: pAuth,
        api_key: pKey || undefined,
      });
      setPName(""); setPKey(""); setPUrl("");
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const decide = async (id: string, approved: boolean) => {
    await api.approvals.decide(id, approved);
    refresh();
  };

  return (
    <div className="content">
      <h2>Settings</h2>
      {error && <div className="error-banner">{error}</div>}

      <section className="card" style={{ marginBottom: 16 }}>
        <h3>Appearance</h3>
        <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
          <button className={`btn ${theme === "light" ? "primary" : ""}`} onClick={() => setTheme("light")}>Light</button>
          <button className={`btn ${theme === "dark" ? "primary" : ""}`} onClick={() => setTheme("dark")}>Dark</button>
        </div>
      </section>

      <section className="card" style={{ marginBottom: 16 }}>
        <h3>Providers</h3>
        <div style={{ display: "grid", gap: 8, marginTop: 8 }}>
          <input className="input" placeholder="Name" value={pName} onChange={(e) => setPName(e.target.value)} />
          <select className="select" value={pKind} onChange={(e) => setPKind(e.target.value)}>
            {["openai","anthropic","openai_compat","ollama","chatgpt","openrouter","deepseek","xai","google","azure","databricks","snowflake"].map((k) => (
              <option key={k} value={k}>{k}</option>
            ))}
          </select>
          <select className="select" value={pAuth} onChange={(e) => setPAuth(e.target.value)}>
            <option value="api_key">api_key</option>
            <option value="oauth">oauth</option>
            <option value="none">none</option>
          </select>
          <input className="input" placeholder="Base URL (optional)" value={pUrl} onChange={(e) => setPUrl(e.target.value)} />
          <input className="input" placeholder="API key (stored securely)" value={pKey} onChange={(e) => setPKey(e.target.value)} type="password" />
          <button className="btn primary" onClick={createProvider}>Add Provider</button>
        </div>
        <div style={{ marginTop: 12 }}>
          {providers.map((p) => (
            <div key={p.provider.id} style={{ padding: "8px 0", borderBottom: "1px solid var(--color-border)" }}>
              <strong>{p.provider.name}</strong> <span className="badge">{p.provider.kind}</span>
              <div style={{ fontSize: "var(--text-xs)", color: "var(--color-text-secondary)" }}>
                secret configured: {p.has_configured_secret ? "yes" : "no"}
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="card" style={{ marginBottom: 16 }}>
        <h3>Pending Approvals</h3>
        {approvals.length === 0 && <div className="empty-state">No pending approvals.</div>}
        {approvals.map((a) => (
          <div key={a.id} className="card" style={{ marginTop: 8 }}>
            <div><strong>{a.kind}</strong> — {a.summary}</div>
            {a.command && <pre>{a.command}</pre>}
            {a.cwd && <div style={{ fontSize: "var(--text-xs)", color: "var(--color-text-secondary)" }}>cwd: {a.cwd}</div>}
            <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
              <button className="btn primary" onClick={() => decide(a.id, true)}>Approve</button>
              <button className="btn danger" onClick={() => decide(a.id, false)}>Deny</button>
            </div>
          </div>
        ))}
      </section>

      <section className="card">
        <h3>Settings (raw)</h3>
        <table className="table">
          <thead><tr><th>Key</th><th>Value</th></tr></thead>
          <tbody>
            {settings.map((s) => (
              <tr key={s.key}><td>{s.key}</td><td><code>{s.value}</code></td></tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}