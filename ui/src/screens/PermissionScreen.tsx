import React, { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";

export default function PermissionScreen() {
  const { projectId } = useApp();
  const [rules, setRules] = useState<T.CommandRule[]>([]);
  const [toolPerms, setToolPerms] = useState<T.ToolPermissionStatus[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [pattern, setPattern] = useState("");
  const [action, setAction] = useState("ask");

  const refresh = () => {
    if (!projectId) return;
    apiFetchGet<T.CommandRule[]>(`/api/projects/${projectId}/command-rules`).then(setRules).catch((e) => setError(String(e)));
    apiFetchGet<T.ToolPermissionStatus[]>(`/api/projects/${projectId}/tool-permissions`).then(setToolPerms).catch(() => {});
  };
  useEffect(refresh, [projectId]);

  const addRule = async () => {
    if (!projectId || !pattern.trim()) return;
    try {
      await fetch(`/api/projects/${projectId}/command-rules`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ pattern: pattern.trim(), action }),
      });
      setPattern("");
      refresh();
    } catch (e) { setError(String(e)); }
  };

  const setToolPerm = async (toolId: string, level: string) => {
    if (!projectId) return;
    try {
      await fetch(`/api/projects/${projectId}/tool-permissions`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ extension_tool_id: toolId, level }),
      });
      refresh();
    } catch (e) { setError(String(e)); }
  };

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Permissions</h2>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
      {!projectId && <div className="text-text-secondary text-sm">Select a project first.</div>}
      {projectId && (
        <>
          <Card className="mb-4">
            <CardHeader><CardTitle className="text-text-primary text-base">Command Rules</CardTitle></CardHeader>
            <CardContent>
              <div className="flex gap-2 mb-3">
                <Input placeholder="Pattern (command)" value={pattern} onChange={(e) => setPattern(e.target.value)} />
                <select className="px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm" value={action} onChange={(e) => setAction(e.target.value)}>
                  <option value="allow">allow</option>
                  <option value="deny">deny</option>
                  <option value="ask">ask</option>
                </select>
                <Button variant="default" size="sm" onClick={addRule}>Add</Button>
              </div>
              <table className="w-full text-sm">
                <thead><tr className="border-b border-border-primary text-text-secondary"><th className="text-left py-2 px-3">Pattern</th><th className="text-left py-2 px-3">Action</th></tr></thead>
                <tbody>
                  {rules.map((r) => (
                    <tr key={r.id} className="border-b border-border-primary">
                      <td className="py-2 px-3 text-text-primary"><code className="text-xs">{r.pattern}</code></td>
                      <td className="py-2 px-3 text-text-secondary">{r.action}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {rules.length === 0 && <div className="text-text-secondary text-sm mt-2">No command rules.</div>}
            </CardContent>
          </Card>

          <Card>
            <CardHeader><CardTitle className="text-text-primary text-base">Tool Permissions</CardTitle></CardHeader>
            <CardContent>
              <table className="w-full text-sm">
                <thead><tr className="border-b border-border-primary text-text-secondary"><th className="text-left py-2 px-3">Extension</th><th className="text-left py-2 px-3">Tool</th><th className="text-left py-2 px-3">Level</th></tr></thead>
                <tbody>
                  {toolPerms.map((t) => (
                    <tr key={t.extension_tool_id} className="border-b border-border-primary">
                      <td className="py-2 px-3 text-text-primary">{t.extension_name}</td>
                      <td className="py-2 px-3 text-text-secondary">{t.tool_name}{t.is_read_only ? " (ro)" : ""}</td>
                      <td className="py-2 px-3">
                        <select className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-xs" value={t.level || "ask_before"} onChange={(e) => setToolPerm(t.extension_tool_id, e.target.value)}>
                          <option value="always_allow">always_allow</option>
                          <option value="ask_before">ask_before</option>
                          <option value="never_allow">never_allow</option>
                        </select>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {toolPerms.length === 0 && <div className="text-text-secondary text-sm mt-2">No tool permissions.</div>}
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}

async function apiFetchGet<T>(path: string): Promise<T> {
  const res = await fetch(path);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}