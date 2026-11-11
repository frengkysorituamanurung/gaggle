import React, { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type * as T from "../../types";
import { Button } from "../ui/Button";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card";
import { EmptyState } from "../EmptyState";
import { LoadingState } from "../LoadingState";
import { ErrorState } from "../ErrorState";

export function ExtensionsView() {
  const [inv, setInv] = useState<T.ExtensionsInventory | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = () => {
    setLoading(true);
    api.extensions.inventory().then(setInv).catch((e) => setError(String(e))).finally(() => setLoading(false));
  };
  useEffect(refresh, []);

  if (loading) return <LoadingState />;
  if (error) return <ErrorState message={error} onRetry={refresh} />;
  if (!inv || inv.extensions.length === 0) return <EmptyState title="No extensions" description="Install bundled extensions to get started." />;

  return (
    <div className="space-y-3">
      {inv.extensions.map((e) => (
        <Card key={e.id}>
          <CardHeader>
            <CardTitle className="text-text-primary text-base flex items-center justify-between">
              <span>{e.name}</span>
              <span className="text-xs px-2 py-0.5 rounded-full bg-surface-2 text-text-secondary">{e.kind}</span>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-xs text-text-secondary mb-2">{e.description}</div>
            <div className="text-xs text-text-tertiary mb-2">Tools: {e.tool_count}</div>
            <Button
              size="sm"
              variant={e.enabled ? "destructive" : "default"}
              onClick={async () => {
                await api.extensions.setEnabled(e.id, !e.enabled);
                refresh();
              }}
            >
              {e.enabled ? "Disable" : "Enable"}
            </Button>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

export default ExtensionsView;