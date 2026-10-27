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

  const inputCls = "w-full px-3 py-2 rounded-md border border-border bg-background text-text text-sm";

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text mb-4">Settings</h2>
      {error && <div className="text-danger text-sm mb-2">{error}</div>}

      <section className="bg-surface border border-border rounded-xl p-4 mb-4">
        <h3 className="font-medium text-text mb-2">Appearance</h3>
        <div className="flex gap-2">
          <button className={`px-3 py-2 rounded-md border border-border text-sm ${theme === "light" ? "bg-accent text-background" : "bg-background text-text hover:bg-surface-2"}`} onClick={() => setTheme("light")}>Light</button>
          <button className={`px-3 py-2 rounded-md border border-border text-sm ${theme === "dark" ? "bg-accent text-background" : "bg-background text-text hover:bg-surface-2"}`} onClick={() => setTheme("dark")}>Dark</button>
        </div>
      </section>

      <section className="bg-surface border border-border rounded-xl p-4 mb-4">
        <h3 className="font-medium text-text mb-2">Providers</h3>
        <div className="flex flex-col gap-2">
          <input className={inputCls} placeholder="Name" value={pName} onChange={(e) => setPName(e.target.value)} />
          <select className={inputCls} value={pKind} onChange={(e) => setPKind(e.target.value)}>
            {["openai","anthropic","openai_compat","ollama","chatgpt","openrouter","deepseek","xai","google","azure","databricks","snowflake"].map((k) => (
              <option key={k} value={k}>{k}</option>
            ))}
          </select>
          <select className={inputCls} value={pAuth} onChange={(e) => setPAuth(e.target.value)}>
            <option value="api_key">api_key</option>
            <option value="oauth">oauth</option>
            <option value="none">none</option>
          </select>
          <input className={inputCls} placeholder="Base URL (optional)" value={pUrl} onChange={(e) => setPUrl(e.target.value)} />
          <input className={inputCls} placeholder="API key (stored securely)" value={pKey} onChange={(e) => setPKey(e.target.value)} type="password" />
          <button className="px-3 py-2 rounded-md bg-accent text-background font-medium hover:bg-accent-hover text-sm" onClick={createProvider}>Add Provider</button>
        </div>
        <div className="mt-3">
          {providers.map((p) => (
            <div key={p.provider.id} className="py-2 border-b border-border last:border-0">
              <span className="font-medium text-text">{p.provider.name}</span>{" "}
              <span className="text-xs px-2 py-0.5 rounded-full bg-surface-2 text-text-secondary">{p.provider.kind}</span>
              <div className="text-xs text-text-secondary">secret configured: {p.has_configured_secret ? "yes" : "no"}</div>
            </div>
          ))}
        </div>
      </section>

      <section className="bg-surface border border-border rounded-xl p-4 mb-4">
        <h3 className="font-medium text-text mb-2">Pending Approvals</h3>
        {approvals.length === 0 && <div className="text-text-secondary text-sm">No pending approvals.</div>}
        {approvals.map((a) => (
          <div key={a.id} className="bg-surface-2 border border-border rounded-xl p-3 mt-2">
            <div className="text-text"><strong>{a.kind}</strong> — {a.summary}</div>
            {a.command && <pre className="mt-1 text-xs bg-code-bg p-2 rounded-md overflow-x-auto">{a.command}</pre>}
            {a.cwd && <div className="text-xs text-text-secondary">cwd: {a.cwd}</div>}
            <div className="flex gap-2 mt-2">
              <button className="px-3 py-1 rounded-md bg-accent text-background text-sm" onClick={() => decide(a.id, true)}>Approve</button>
              <button className="px-3 py-1 rounded-md border border-danger text-danger text-sm" onClick={() => decide(a.id, false)}>Deny</button>
            </div>
          </div>
        ))}
      </section>

      <section className="bg-surface border border-border rounded-xl p-4">
        <h3 className="font-medium text-text mb-2">Settings (raw)</h3>
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border text-text-secondary">
              <th className="text-left py-2 px-3">Key</th>
              <th className="text-left py-2 px-3">Value</th>
            </tr>
          </thead>
          <tbody>
            {settings.map((s) => (
              <tr key={s.key} className="border-b border-border">
                <td className="py-2 px-3 text-text">{s.key}</td>
                <td className="py-2 px-3"><code className="text-xs">{s.value}</code></td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}