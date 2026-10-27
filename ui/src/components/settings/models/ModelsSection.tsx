import React, { useEffect, useState } from "react";
import { api } from "../../../lib/api";
import type * as T from "../../../types";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";

export default function ModelsSection() {
  const [providers, setProviders] = useState<T.ProviderStatus[]>([]);
  const [models, setModels] = useState<T.Model[]>([]);
  useEffect(() => {
    api.providers.list().then(async (ps) => {
      setProviders(ps);
      const all: T.Model[] = [];
      for (const p of ps) { try { all.push(...(await api.providers.models(p.provider.id))); } catch { /* ignore */ } }
      setModels(all);
    }).catch(() => {});
  }, []);
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Models</CardTitle></CardHeader>
      <CardContent>
        <div className="text-sm text-text-secondary mb-2">Configured models ({models.length}).</div>
        <ul className="text-sm">
          {models.map((m) => (
            <li key={m.id} className="py-1 border-b border-border-primary last:border-0">
              <span className="text-text-primary">{m.display_name}</span>
              <span className="ml-2 text-xs text-text-tertiary">{m.model_key}</span>
            </li>
          ))}
        </ul>
        {models.length === 0 && <div className="text-text-secondary text-sm">No models. Add a provider in the form below.</div>}
      </CardContent>
    </Card>
  );
}