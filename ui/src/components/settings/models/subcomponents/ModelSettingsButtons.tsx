import React from "react";
import { Button } from "../../../ui/Button";

export function ModelSettingsButtons({ onRefresh, onAdd }: { onRefresh?: () => void; onAdd?: () => void }) {
  return (
    <div className="flex gap-2">
      {onRefresh && <Button size="sm" variant="secondary" onClick={onRefresh}>Refresh</Button>}
      {onAdd && <Button size="sm" variant="default" onClick={onAdd}>Add Model</Button>}
    </div>
  );
}

export default ModelSettingsButtons;