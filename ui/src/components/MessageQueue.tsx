import React from "react";
import { Button } from "./ui/Button";

export type QueuedMessage = {
  id: string;
  content: string;
  images?: { dataUrl: string; mimeType: string }[];
};

export default function MessageQueue({
  count,
  items = [],
  onClear,
  onSteer,
}: {
  count: number;
  items?: QueuedMessage[];
  onClear?: () => void;
  onSteer?: (id: string) => void;
}) {
  if (count <= 0 && items.length === 0) return null;
  const list = items.length ? items : [];
  return (
    <div className="flex flex-col gap-1 text-xs text-text-secondary border border-border-primary rounded-md p-2 bg-background-secondary">
      <div className="flex items-center justify-between">
        <span>{count || list.length} message(s) queued</span>
        {onClear && (
          <Button variant="ghost" size="sm" onClick={onClear}>
            Clear
          </Button>
        )}
      </div>
      {list.map((m) => (
        <div key={m.id} className="flex items-center justify-between gap-2">
          <span className="truncate flex-1">{m.content}</span>
          {onSteer && (
            <Button variant="ghost" size="sm" onClick={() => onSteer(m.id)}>
              Send now
            </Button>
          )}
        </div>
      ))}
    </div>
  );
}