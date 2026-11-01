import React from "react";
import { Button } from "../../ui/Button";

export function ConfigureApproveMode({ mode, onChange }: { mode?: string; onChange?: (m: string) => void }) {
  return (
    <div className="flex items-center gap-2">
      <span className="text-sm text-text-primary">Approval mode:</span>
      <select
        className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
        value={mode || "ask"}
        onChange={(e) => onChange?.(e.target.value)}
      >
        <option value="ask">Ask</option>
        <option value="auto">Auto</option>
        <option value="deny">Deny</option>
      </select>
    </div>
  );
}

export default ConfigureApproveMode;><|end|><|start|>write to=assistant<|channel|>analysis<|message|>Created file successfully: ui/src/components/settings/mode/ConfigureApproveMode.tsx