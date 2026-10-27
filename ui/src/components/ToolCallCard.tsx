import React, { useState } from "react";
import { cn } from "../lib/utils";

export function ToolCallCard({ kind, content }: { kind: "tool_call" | "tool_result"; content: string | null }) {
  const [open, setOpen] = useState(false);
  const label = kind === "tool_call" ? "Tool call" : "Tool result";
  return (
    <div className="mt-2 rounded-md border border-border-primary bg-background-tertiary text-sm overflow-hidden">
      <button
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center justify-between px-3 py-2 text-left hover:bg-background-tertiary/80"
      >
        <span className="font-medium text-text-primary">{label}</span>
        <span className="text-text-tertiary text-xs">{open ? "▾" : "▸"}</span>
      </button>
      {open && (
        <pre className="px-3 py-2 text-xs overflow-x-auto border-t border-border-primary bg-background-primary text-text-primary">
          {content}
        </pre>
      )}
    </div>
  );
}