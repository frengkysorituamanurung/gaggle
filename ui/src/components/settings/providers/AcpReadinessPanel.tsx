import React from "react";

export function AcpReadinessPanel() {
  return (
    <div className="rounded-md border border-border-primary bg-background-secondary p-3 text-sm">
      <div className="font-medium text-text-primary mb-1">ACP Readiness</div>
      <div className="text-xs text-text-secondary">Gaggle uses REST/SSE backend (not ACP). Status: ready.</div>
    </div>
  );
}

export default AcpReadinessPanel;