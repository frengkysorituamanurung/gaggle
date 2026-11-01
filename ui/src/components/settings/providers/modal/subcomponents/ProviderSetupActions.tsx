import React from "react";
import { Button } from "../../../ui/Button";
export function ProviderSetupActions({ onSave, onCancel }: { onSave?: () => void; onCancel?: () => void }) {
  return (
    <div className="flex justify-end gap-2">
      {onCancel && <Button size="sm" variant="secondary" onClick={onCancel}>Cancel</Button>}
      {onSave && <Button size="sm" variant="default" onClick={onSave}>Save</Button>}
    </div>
  );
}
export default ProviderSetupActions;