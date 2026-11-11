import React, { useState, useEffect } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { ExtensionConfigFields } from "./ExtensionConfigFields";
import { HeadersSection } from "./HeadersSection";
import { EnvVarsSection } from "./EnvVarsSection";
import type * as T from "../../types";

export function ExtensionModal({
  open,
  onClose,
  initial,
  onSave,
}: {
  open: boolean;
  onClose: () => void;
  initial?: T.Extension | null;
  onSave?: (data: { name: string; command?: string; url?: string }) => void;
}) {
  const [name, setName] = useState("");
  const [command, setCommand] = useState("");
  const [url, setUrl] = useState("");
  const [headers, setHeaders] = useState<{ key: string; value: string }[]>([]);
  const [envVars, setEnvVars] = useState<{ key: string; value: string }[]>([]);

  useEffect(() => {
    if (open) {
      setName(initial?.name || "");
      setCommand(initial?.command || "");
      setUrl(initial?.url || "");
      setHeaders([]);
      setEnvVars([]);
    }
  }, [open, initial]);

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{initial?.id ? "Edit Extension" : "New Extension"}</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <ExtensionConfigFields
            name={name}
            onNameChange={setName}
            command={command}
            onCommandChange={setCommand}
            url={url}
            onUrlChange={setUrl}
          />
          <HeadersSection headers={headers} onChange={setHeaders} />
          <EnvVarsSection envVars={envVars} onChange={setEnvVars} />
          <div className="flex justify-end gap-2">
            <Button variant="secondary" size="sm" onClick={onClose}>Cancel</Button>
            <Button variant="default" size="sm" onClick={() => { onSave?.({ name, command, url }); onClose(); }}>
              Save
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default ExtensionModal;