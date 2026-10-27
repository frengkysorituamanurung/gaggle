import React, { useState } from "react";
import { cn } from "../lib/utils";

export function ThinkingBlock({ content }: { content: string | null }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="mt-2 rounded-md border border-border-primary bg-background-secondary text-sm">
      <button
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left text-text-secondary hover:bg-background-tertiary/60"
      >
        <span className="text-xs">{open ? "▾" : "▸"}</span>
        <span className="text-xs uppercase tracking-wide">Reasoning</span>
      </button>
      {open && (
        <pre className="px-3 py-2 text-xs overflow-x-auto border-t border-border-primary bg-background-primary text-text-primary whitespace-pre-wrap">
          {content}
        </pre>
      )}
    </div>
  );
}