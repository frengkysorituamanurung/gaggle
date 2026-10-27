import React, { useEffect, useState } from "react";
import { useApp } from "../../../lib/store";
import { api } from "../../../lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";

export default function GoosehintsSection() {
  const { projectId } = useApp();
  const [content, setContent] = useState("");
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    if (projectId) api.settings.list().then((s) => {
      const v = s.find((x) => x.key === `goosehints:${projectId}`)?.value;
      if (v) setContent(v);
    }).catch(() => {});
  }, [projectId]);
  const save = async () => {
    if (!projectId) return;
    setSaving(true);
    try { await api.settings.upsert(`goosehints:${projectId}`, content); } catch { /* ignore */ } finally { setSaving(false); }
  };
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Goosehints</CardTitle></CardHeader>
      <CardContent>
        <p className="text-sm text-text-secondary mb-2">Project-specific hints injected into the system prompt.</p>
        <textarea className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm min-h-[120px]" value={content} onChange={(e) => setContent(e.target.value)} />
        <Button variant="default" size="sm" className="mt-2" onClick={save} disabled={saving}>{saving ? "Saving…" : "Save"}</Button>
      </CardContent>
    </Card>
  );
}