import React from "react";
import type * as T from "../../../../types";
export function ExtensionItem({ ext, onToggle }: { ext: T.Extension; onToggle?: (id: string, enabled: boolean) => void }) {
  return (
    <div className="flex items-center justify-between rounded-md border border-border-primary bg-background-secondary p-2 text-sm">
      <span className="text-text-primary">{ext.name}</span>
      <button className="text-xs text-text-primary underline" onClick={() => onToggle?.(ext.id, !ext.enabled)}>
        {ext.enabled ? "Disable" : "Enable"}
      </button>
    </div>
  );
}
export default ExtensionItem;