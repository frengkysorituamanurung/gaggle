import React from "react";
import { useApp } from "../../lib/store";
import type * as T from "../../types";
import { Button } from "../ui/Button";

export function SessionListView({
  sessions = [],
  onSelect,
  onRename,
}: {
  sessions?: T.Chat[];
  onSelect?: (s: T.Chat) => void;
  onRename?: (s: T.Chat) => void;
}) {
  if (sessions.length === 0) {
    return <div className="text-sm text-text-secondary">No sessions.</div>;
  }
  return (
    <div className="space-y-1">
      {sessions.map((s) => (
        <div
          key={s.id}
          className="flex items-center justify-between rounded-md border border-border-primary bg-background-secondary px-3 py-2 cursor-pointer hover:bg-background-tertiary"
          onClick={() => onSelect?.(s)}
        >
          <div className="truncate text-sm text-text-primary">{s.title}</div>
          {onRename && (
            <Button size="sm" variant="ghost" onClick={(e) => { e.stopPropagation(); onRename(s); }}>
              Rename
            </Button>
          )}
        </div>
      ))}
    </div>
  );
}

export default SessionListView;