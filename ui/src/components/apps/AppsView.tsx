import React, { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type * as T from "../../types";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card";

export default function AppsView() {
  const [exts, setExts] = useState<T.ExtensionInventoryItem[]>([]);
  useEffect(() => { api.extensions.inventory().then((i) => setExts(i.extensions)).catch(() => {}); }, []);
  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Apps</h2>
      <div className="grid gap-3">
        {exts.map((e) => (
          <Card key={e.id}>
            <CardHeader><CardTitle className="text-text-primary text-base">{e.name}</CardTitle></CardHeader>
            <CardContent>
              <div className="text-xs text-text-tertiary">{e.description}</div>
              <div className="text-sm text-text-secondary mt-1">Tools: {e.tool_count} · {e.enabled ? "enabled" : "disabled"}</div>
              {e.url && <a className="text-sm text-accent hover:underline" href={e.url} target="_blank" rel="noreferrer">Open</a>}
            </CardContent>
          </Card>
        ))}
      </div>
      {exts.length === 0 && <div className="text-text-secondary text-center py-12">No apps/extensions.</div>}
    </div>
  );
}