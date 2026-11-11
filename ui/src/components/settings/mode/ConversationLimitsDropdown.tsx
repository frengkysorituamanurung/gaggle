import React from "react";
import { Button } from "../../ui/Button";

export function ConversationLimitsDropdown({ value, onChange }: { value?: number; onChange?: (v: number) => void }) {
  return (
    <select
      className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
      value={value ?? 10}
      onChange={(e) => onChange?.(Number(e.target.value))}
    >
      {[5, 10, 20, 50].map((n) => (
        <option key={n} value={n}>{n} messages</option>
      ))}
    </select>
  );
}

export default ConversationLimitsDropdown;