import React from "react";

export function ModelSettingsPanel({ title, children }: { title?: string; children?: React.ReactNode }) {
  return (
    <div className="rounded-md border border-border-primary bg-background-secondary p-3 text-sm">
      {title && <div className="font-medium text-text-primary mb-2">{title}</div>}
      {children}
    </div>
  );
}

export default ModelSettingsPanel;