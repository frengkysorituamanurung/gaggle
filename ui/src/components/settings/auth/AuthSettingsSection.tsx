import React, { useEffect, useState } from "react";
import { api } from "../../../lib/api";
import type * as T from "../../../types";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";
import { Button } from "../../ui/Button";

export default function AuthSettingsSection() {
  const [providers, setProviders] = useState<T.ProviderStatus[]>([]);
  useEffect(() => { api.providers.list().then(setProviders).catch(() => {}); }, []);
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Provider Credentials</CardTitle></CardHeader>
      <CardContent>
        <p className="text-sm text-text-secondary mb-3">Manage authentication for model providers.</p>
        {providers.map((p) => (
          <div key={p.provider.id} className="py-2 border-b border-border-primary last:border-0 flex items-center justify-between">
            <div>
              <span className="font-medium text-text-primary">{p.provider.name}</span>
              <span className="ml-2 text-xs px-2 py-0.5 rounded-full bg-surface-2 text-text-secondary">{p.provider.kind}</span>
              <div className="text-xs text-text-secondary">secret configured: {p.has_configured_secret ? "yes" : "no"}</div>
            </div>
            {p.provider.kind === "chatgpt" && (
              <Button size="sm" variant="secondary" onClick={async () => {
                try {
                  const r = await api.providers.startChatgptOAuth(p.provider.id);
                  if (r.authorization_url) window.open(r.authorization_url, "_blank");
                } catch { /* ignore */ }
              }}>Sign in</Button>
            )}
          </div>
        ))}
        {providers.length === 0 && <div className="text-text-secondary text-sm">No providers configured.</div>}
      </CardContent>
    </Card>
  );
}