import React, { useState } from "react";
import { cn } from "../lib/utils";

export function ToolCallCard({
  label,
  status,
  content,
  durationMs,
}: {
  label: string;
  status?: string;
  content?: string | null;
  durationMs?: number;
}) {
  const [open, setOpen] = useState(false);
  const statusColor =
    status === "ok" || status === "completed"
      ? "text-text-success"
      : status === "error" || status === "failed"
        ? "text-text-danger"
        : "text-text-secondary";

  return (
    <div className="mt-2 rounded-md border border-border-primary bg-background-tertiary text-sm overflow-hidden">
      <button
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center justify-between px-3 py-2 text-left hover:bg-background-tertiary/80"
      >
        <span className="font-medium text-text-primary">{label}</span>
        <span className="flex items-center gap-2 text-xs">
          {status && <span className={statusColor}>{status}</span>}
          {durationMs != null && <span className="text-text-tertiary">{durationMs}ms</span>}
          <span className="text-text-tertiary">{open ? "▾" : "▸"}</span>
        </span>
      </button>
      {open && content != null && (
        <pre className="px-3 py-2 text-xs overflow-x-auto border-t border-border-primary bg-background-primary text-text-primary whitespace-pre-wrap">
          {content}
        </pre>
      )}
    </div>
  );
}
