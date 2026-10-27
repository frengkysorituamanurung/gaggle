import React from "react";
import { useApp } from "../../../lib/store";
import { api } from "../../../lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";
import { Switch } from "../../ui/Switch";

export default function ModeSection() {
  const { projectId } = useApp();
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Default Mode</CardTitle></CardHeader>
      <CardContent>
        <p className="text-sm text-text-secondary">Choose the default chat mode for new chats in this project.</p>
        <div className="flex gap-2 mt-3">
          {["local", "worktree", "readonly"].map((m) => (
            <button key={m} className="px-3 py-1.5 rounded-md border border-border-primary text-sm text-text-primary hover:bg-background-secondary" onClick={() => projectId && api.projects.update(projectId, { default_mode: m }).catch(() => {})}>
              {m}
            </button>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}