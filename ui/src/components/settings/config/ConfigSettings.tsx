import React from "react";
import { Button } from "../../ui/Button";

export function ConfigSettings() {
  return (
    <div className="rounded-md border border-border-primary bg-background-secondary p-3 text-sm">
      <div className="font-medium text-text-primary mb-2">Configuration</div>
      <p className="text-xs text-text-secondary mb-2">Manage raw configuration values.</p>
      <Button size="sm" variant="secondary">Open config</Button>
    </div>
  );
}

export default ConfigSettings;