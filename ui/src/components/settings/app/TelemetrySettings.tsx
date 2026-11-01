import React from "react";
import { Switch } from "../../ui/Switch";

export function TelemetrySettings() {
  const [enabled, setEnabled] = React.useState(false);
  return (
    <div className="flex items-center justify-between rounded-md border border-border-primary bg-background-secondary p-3 text-sm">
      <div>
        <div className="font-medium text-text-primary">Telemetry</div>
        <div className="text-xs text-text-secondary">Send anonymous usage data.</div>
      </div>
      <Switch checked={enabled} onCheckedChange={setEnabled} />
    </div>
  );
}

export default TelemetrySettings;