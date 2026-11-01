import React from "react";
import { Button } from "../../ui/Button";

export function ModelsBottomBar({ onRefresh }: { onRefresh?: () => void }) {
  return (
    <div className="flex items-center justify-between border-t border-border-primary px-4 py-2">
      <span className="text-xs text-text-secondary">Models</span>
      <Button size="sm" variant="secondary" onClick={onRefresh}>Refresh</Button>
    </div>
  );
}

export default ModelsBottomBar;