import React, { useState } from "react";
import { Button } from "../ui/Button";
import { Puzzle } from "lucide-react";
import type * as T from "../../types";

export function ExtensionMenu({
  extensions = [],
  onToggle,
}: {
  extensions?: T.Extension[];
  onToggle?: (id: string, enabled: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative">
      <Button variant="ghost" size="sm" onClick={() => setOpen((v) => !v)} title="Extensions">
        <Puzzle className="w-4 h-4" />
      </Button>
      {open && (
        <div className="absolute bottom-full mb-2 z-50 w-64 max-h-64 overflow-y-auto rounded-md border border-border-primary bg-background-primary p-2 shadow-lg">
          {extensions.length === 0 && <div className="text-xs text-text-secondary p-2">No extensions.</div>}
          {extensions.map((e) => (
            <label key={e.id} className="flex items-center gap-2 px-2 py-1 text-sm text-text-primary hover:bg-background-secondary rounded cursor-pointer">
              <input
                type="checkbox"
                checked={!!e.enabled}
                onChange={(ev) => onToggle?.(e.id, ev.target.checked)}
              />
              <span className="truncate">{e.name}</span>
            </label>
          ))}
          <div className="mt-2 flex justify-end">
            <Button size="sm" variant="secondary" onClick={() => setOpen(false)}>Close</Button>
          </div>
        </div>
      )}
    </div>
  );
}

export default ExtensionMenu;