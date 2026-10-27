import React, { useEffect, useState } from "react";
import { api } from "../../../lib/api";
import type * as T from "../../../types";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";

export default function ModelsSection() {
  const [providers, setProviders] = useState<T.ProviderStatus[]>([]);
  const [models, setModels] = useState<T.Model[]>([]);
  const [refreshing, setRefreshing] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [newProviderId, setNewProviderId] = useState("");
  const [newModelKey, setNewModelKey] = useState("");
  const [newModelName, setNewModelName] = useState("");
  const [newContextWindow, setNewContextWindow] = useState("128000");

  const load = async () => {
    try {
      const ps = await api.providers.list();
      setProviders(ps);
      const all: T.Model[] = [];
      for (const p of ps) {
        try {
          all.push(...(await api.providers.models(p.provider.id)));
        } catch {
          /* ignore */
        }
      }
      setModels(all);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    load();
  }, []);

  const refreshProvider = async (id: string) => {
    setRefreshing(id);
    try {
      const refreshed = await api.providers.refreshModels(id);
      setModels((prev) => [...prev.filter((m) => m.provider_id !== id), ...refreshed]);
    } catch (e) {
      setError(String(e));
    } finally {
      setRefreshing(null);
    }
  };

  const addModel = async () => {
    if (!newProviderId || !newModelKey.trim() || !newModelName.trim()) return;
    try {
      const created = await api.providers.createModel(newProviderId, {
        model_key: newModelKey.trim(),
        display_name: newModelName.trim(),
        context_window: Number(newContextWindow) || 128000,
        supports_tools: true,
      });
      setModels((prev) => [...prev, created]);
      setNewModelKey("");
      setNewModelName("");
    } catch (e) {
      setError(String(e));
    }
  };

  const modelsByProvider = (id: string) => models.filter((m) => m.provider_id === id);

  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Models</CardTitle></CardHeader>
      <CardContent>
        {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
        <div className="text-sm text-text-secondary mb-2">Configured models ({models.length}).</div>

        <div className="mb-4 flex flex-col gap-2">
          <div className="text-xs font-semibold text-text-secondary uppercase">Add model manually</div>
          <select
            className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
            value={newProviderId}
            onChange={(e) => setNewProviderId(e.target.value)}
          >
            <option value="">Select provider</option>
            {providers.map((p) => (
              <option key={p.provider.id} value={p.provider.id}>{p.provider.name}</option>
            ))}
          </select>
          <Input placeholder="Model key" value={newModelKey} onChange={(e) => setNewModelKey(e.target.value)} />
          <Input placeholder="Display name" value={newModelName} onChange={(e) => setNewModelName(e.target.value)} />
          <Input placeholder="Context window" value={newContextWindow} onChange={(e) => setNewContextWindow(e.target.value)} />
          <Button size="sm" onClick={addModel} disabled={!newProviderId || !newModelKey.trim() || !newModelName.trim()}>
            Add model
          </Button>
        </div>

        {providers.map((p) => (
          <div key={p.provider.id} className="mb-4">
            <div className="flex items-center justify-between mb-1">
              <span className="font-medium text-text-primary">{p.provider.name}</span>
              <Button size="sm" variant="secondary" onClick={() => refreshProvider(p.provider.id)} disabled={refreshing === p.provider.id}>
                {refreshing === p.provider.id ? "Refreshing…" : "Refresh"}
              </Button>
            </div>
            <ul className="text-sm">
              {modelsByProvider(p.provider.id).map((m) => (
                <li key={m.id} className="py-1 border-b border-border-primary last:border-0">
                  <span className="text-text-primary">{m.display_name}</span>
                  <span className="ml-2 text-xs text-text-tertiary">{m.model_key}</span>
                  <span className="ml-2 text-xs text-text-tertiary">ctx {m.context_window}</span>
                </li>
              ))}
              {modelsByProvider(p.provider.id).length === 0 && (
                <li className="py-1 text-text-secondary text-sm">No models.</li>
              )}
            </ul>
          </div>
        ))}
        {providers.length === 0 && <div className="text-text-secondary text-sm">No providers configured.</div>}
      </CardContent>
    </Card>
  );
}
