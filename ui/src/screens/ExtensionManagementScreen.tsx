import React, { useEffect, useState } from "react";
import { api } from "../lib/api";
import type * as T from "../types";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";
import { Button } from "../components/ui/Button";

export default function ExtensionManagementScreen() {
  const [extensions, setExtensions] = useState<T.Extension[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const inv = await api.extensions.inventory();
      setExtensions(inv.extensions || []);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const toggle = async (ext: T.Extension) => {
    try {
      await api.extensions.setEnabled(ext.id, !ext.enabled);
      await load();
    } catch (e) { setError(String(e)); }
  };

  const discover = async (id: string) => {
    try {
      await api.extensions.discover(id);
      await load();
    } catch (e) { setError(String(e)); }
  };

  const installBundled = async () => {
    try {
      await api.extensions.installBundled();
      await load();
    } catch (e) { setError(String(e)); }
  };

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Extension Management</h2>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
      <Button size="sm" onClick={installBundled} className="mb-4">
        Install Bundled Extensions
      </Button>

      {loading && <div className="text-text-secondary text-sm">Loading...</div>}

      {extensions.map((ext) => (
        <Card key={ext.id} className="mb-2">
          <CardContent className="p-4">
            <div className="flex items-center justify-between mb-1">
              <span className="font-medium text-text-primary">{ext.name}</span>
              <span className={`text-xs px-2 py-0.5 rounded-full ${ext.enabled ? "bg-green-100 text-green-800" : "bg-gray-100 text-gray-800"}`}>
                {ext.enabled ? "Enabled" : "Disabled"}
              </span>
            </div>
            {ext.description && <p className="text-sm text-text-secondary mb-2">{ext.description}</p>}
            <div className="text-xs text-text-tertiary mb-2">
              Kind: {ext.kind} | Source: {ext.source}
            </div>
            <div className="flex gap-2">
              <Button size="sm" variant="secondary" onClick={() => toggle(ext)}>
                {ext.enabled ? "Disable" : "Enable"}
              </Button>
              <Button size="sm" variant="secondary" onClick={() => discover(ext.id)}>
                Discover Tools
              </Button>
            </div>
          </CardContent>
        </Card>
      ))}
      {extensions.length === 0 && !loading && (
        <div className="text-text-secondary text-sm">No extensions installed.</div>
      )}
    </div>
  );
}