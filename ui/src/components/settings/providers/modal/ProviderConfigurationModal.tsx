import React, { useState, useEffect } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../../../ui/Dialog";
import { Button } from "../../../ui/Button";
import { Input } from "../../../ui/Input";
import { api } from "../../../../lib/api";
import type * as T from "../../../../types";

export function ProviderConfigurationModal({
  open,
  onClose,
  initial,
  onSaved,
}: {
  open: boolean;
  onClose: () => void;
  initial?: T.ProviderStatus | null;
  onSaved?: () => void;
}) {
  const [name, setName] = useState("");
  const [kind, setKind] = useState("openai");
  const [baseUrl, setBaseUrl] = useState("");
  const [authKind, setAuthKind] = useState("api_key");
  const [apiKey, setApiKey] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setName(initial?.provider.name || "");
      setKind(initial?.provider.kind || "openai");
      setBaseUrl(initial?.provider.base_url || "");
      setAuthKind(initial?.provider.auth_kind || "api_key");
      setApiKey("");
      setError(null);
    }
  }, [open, initial]);

  const save = async () => {
    if (!name.trim()) return;
    setSaving(true);
    setError(null);
    try {
      if (!initial?.provider.id) {
        await api.providers.create({ kind, name: name.trim(), base_url: baseUrl || undefined, auth_kind: authKind, api_key: apiKey || undefined });
      }
      onSaved?.();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(v: boolean) => !v && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{initial?.provider.id ? "Edit Provider" : "Add Provider"}</DialogTitle>
        </DialogHeader>
        <div className="space-y-2">
          <Input placeholder="Name" value={name} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setName(e.target.value)} />
          <Input placeholder="Kind (openai/anthropic/...)" value={kind} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setKind(e.target.value)} />
          <Input placeholder="Base URL (optional)" value={baseUrl} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setBaseUrl(e.target.value)} />
          <Input placeholder="Auth kind (api_key/oauth)" value={authKind} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setAuthKind(e.target.value)} />
          <Input placeholder="API key (optional)" type="password" value={apiKey} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setApiKey(e.target.value)} />
          {error && <div className="text-text-danger text-sm">{error}</div>}
          <div className="flex justify-end gap-2">
            <Button variant="secondary" size="sm" onClick={onClose}>Cancel</Button>
            <Button variant="default" size="sm" onClick={save} disabled={saving || !name.trim()}>
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default ProviderConfigurationModal;