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
  const [cataloging, setCataloging] = useState<string | null>(null);
  const [catalogs, setCatalogs] = useState<Record<string, string[]>>({});
  const [error, setError] = useState<string | null>(null);

  const [newProviderId, setNewProviderId] = useState("");
  const [newModelKey, setNewModelKey] = useState("");
  const [newModelName, setNewModelName] = useState("");
  const [newContextWindow, setNewContextWindow] = useState("128000");
  const [newSupportsTools, setNewSupportsTools] = useState(true);
  const [newSupportsVision, setNewSupportsVision] = useState(false);
  const [newSupportsReasoning, setNewSupportsReasoning] = useState(false);

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

  const discoverProviderModels = async (id: string) => {
    setCataloging(id);
    setError(null);
    try {
      const catalog = await api.providers.catalog(id);
      setCatalogs((prev) => ({ ...prev, [id]: catalog }));
    } catch (e) {
      setError(String(e));
    } finally {
      setCataloging(null);
    }
  };

  const selectCatalogModel = (providerId: string, modelKey: string) => {
    setNewProviderId(providerId);
    setNewModelKey(modelKey);
    setNewModelName(modelKey);
    setNewContextWindow("");
    setNewSupportsTools(false);
    setNewSupportsVision(false);
    setNewSupportsReasoning(false);
  };

  const addModel = async () => {
    const contextWindow = Number(newContextWindow);
    if (
      !newProviderId ||
      !newModelKey.trim() ||
      !newModelName.trim() ||
      !Number.isInteger(contextWindow) ||
      contextWindow < 1
    ) return;
    try {
      const created = await api.providers.createModel(newProviderId, {
        model_key: newModelKey.trim(),
        display_name: newModelName.trim(),
        context_window: contextWindow,
        supports_tools: newSupportsTools,
        supports_vision: newSupportsVision,
        supports_reasoning: newSupportsReasoning,
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
          <div className="flex flex-wrap gap-4 text-sm text-text-secondary">
            <label className="flex items-center gap-2"><input type="checkbox" checked={newSupportsTools} onChange={(e) => setNewSupportsTools(e.target.checked)} /> Tool calls</label>
            <label className="flex items-center gap-2"><input type="checkbox" checked={newSupportsVision} onChange={(e) => setNewSupportsVision(e.target.checked)} /> Vision</label>
            <label className="flex items-center gap-2"><input type="checkbox" checked={newSupportsReasoning} onChange={(e) => setNewSupportsReasoning(e.target.checked)} /> Reasoning</label>
          </div>
          <Button size="sm" onClick={addModel} disabled={!newProviderId || !newModelKey.trim() || !newModelName.trim() || !Number.isInteger(Number(newContextWindow)) || Number(newContextWindow) < 1}>
            Add model
          </Button>
        </div>

        {providers.map((p) => (
          <div key={p.provider.id} className="mb-4">
            <div className="flex items-center justify-between mb-1">
              <span className="font-medium text-text-primary">{p.provider.name}</span>
              <div className="flex gap-2">
                <Button size="sm" variant="secondary" onClick={() => discoverProviderModels(p.provider.id)} disabled={cataloging === p.provider.id}>
                  {cataloging === p.provider.id ? "Discovering…" : "Discover models"}
                </Button>
                <Button size="sm" variant="secondary" onClick={() => refreshProvider(p.provider.id)} disabled={refreshing === p.provider.id}>
                  {refreshing === p.provider.id ? "Refreshing…" : "Refresh"}
                </Button>
              </div>
            </div>
            {catalogs[p.provider.id] && (
              <>
                <p className="mb-1 text-xs text-text-secondary">
                  Catalog IDs only; verify context window and capabilities before adding.
                </p>
                <ul className="mb-2 text-sm" aria-label={`Discovered models for ${p.provider.name}`}>
                  {catalogs[p.provider.id].map((modelKey) => (
                    <li key={modelKey} className="flex items-center justify-between gap-2 py-1">
                      <span className="text-text-secondary">{modelKey}</span>
                      <Button size="sm" variant="secondary" onClick={() => selectCatalogModel(p.provider.id, modelKey)}>
                        Configure
                      </Button>
                    </li>
                  ))}
                  {catalogs[p.provider.id].length === 0 && (
                    <li className="py-1 text-text-secondary">Provider returned no models.</li>
                  )}
                </ul>
              </>
            )}
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
