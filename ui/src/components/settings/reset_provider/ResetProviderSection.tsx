import React from "react";
import { Button } from "../../ui/Button";

export function ResetProviderSection() {
  return (
    <div className="rounded-md border border-border-primary bg-background-secondary p-3 text-sm">
      <div className="font-medium text-text-primary mb-2">Reset Provider</div>
      <p className="text-xs text-text-secondary mb-2">Reset provider configuration to defaults.</p>
      <Button size="sm" variant="destructive">Reset</Button>
    </div>
  );
}

export default ResetProviderSection;