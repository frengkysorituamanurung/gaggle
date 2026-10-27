import React, { useEffect, useState } from "react";
import { api } from "../../../lib/api";
import type * as T from "../../../types";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";
import { Button } from "../../ui/Button";

export default function LocalInferenceSection() {
  const [ollamaModels, setOllamaModels] = useState<T.Model[]>([]);
  const [loading, setLoading] = useState(false);
  const refresh = async () => {
    setLoading(true);
    try {
      const providers = await api.providers.list();
      const ollama = providers.filter((p) => p.provider.kind === "ollama");
      const all: T.Model[] = [];
      for (const p of ollama) { try { all.push(...(await api.providers.models(p.provider.id))); } catch { /* ignore */ } }
      setOllamaModels(all);
    } finally { setLoading(false); }
  };
  useEffect(() => { refresh(); }, []);
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Local Inference (Ollama)</CardTitle></CardHeader>
      <CardContent>
        <p className="text-sm text-text-secondary mb-2">Models available from local Ollama providers.</p>
        <Button size="sm" variant="secondary" onClick={refresh} disabled={loading}>{loading ? "Loading…" : "Refresh"}</Button>
        <ul className="mt-3 text-sm">
          {ollamaModels.map((m) => (<li key={m.id} className="py-1 border-b border-border-primary last:border-0"><span className="text-text-primary">{m.display_name}</span><span className="ml-2 text-xs text-text-tertiary">{m.model_key}</span></li>))}
        </ul>
        {ollamaModels.length === 0 && !loading && <div className="text-text-secondary text-sm mt-2">No Ollama models. Add an Ollama provider in Models tab.</div>}
      </CardContent>
    </Card>
  );
}