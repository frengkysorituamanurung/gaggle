import React, { useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { api } from "../../lib/api";

export function ExtensionInstallModal({
  open,
  onClose,
  onInstalled,
}: {
  open: boolean;
  onClose: () => void;
  onInstalled?: () => void;
}) {
  const [name, setName] = useState("");
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const install = async () => {
    setInstalling(true);
    setError(null);
    try {
      await api.extensions.installBundled();
      onInstalled?.();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setInstalling(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Install Extension</DialogTitle>
        </DialogHeader>
        <div className="space-y-2">
          <Input placeholder="Extension name (optional)" value={name} onChange={(e) => setName(e.target.value)} />
          {error && <div className="text-text-danger text-sm">{error}</div>}
          <div className="flex justify-end gap-2">
            <Button variant="secondary" size="sm" onClick={onClose}>Cancel</Button>
            <Button variant="default" size="sm" onClick={install} disabled={installing}>
              {installing ? "Installing…" : "Install Bundled"}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default ExtensionInstallModal;