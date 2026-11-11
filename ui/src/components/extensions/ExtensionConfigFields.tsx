import React from "react";
import { Input } from "../ui/Input";

export function ExtensionConfigFields({
  name,
  onNameChange,
  command,
  onCommandChange,
  url,
  onUrlChange,
}: {
  name: string;
  onNameChange: (v: string) => void;
  command?: string;
  onCommandChange?: (v: string) => void;
  url?: string;
  onUrlChange?: (v: string) => void;
}) {
  return (
    <div className="space-y-2">
      <div>
        <label className="text-xs text-text-secondary">Name</label>
        <Input value={name} onChange={(e) => onNameChange(e.target.value)} />
      </div>
      {onCommandChange && (
        <div>
          <label className="text-xs text-text-secondary">Command</label>
          <Input value={command || ""} onChange={(e) => onCommandChange(e.target.value)} />
        </div>
      )}
      {onUrlChange && (
        <div>
          <label className="text-xs text-text-secondary">URL</label>
          <Input value={url || ""} onChange={(e) => onUrlChange(e.target.value)} />
        </div>
      )}
    </div>
  );
}

export default ExtensionConfigFields;