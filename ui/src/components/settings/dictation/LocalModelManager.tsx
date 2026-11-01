import React from "react";
import { Button } from "../../ui/Button";
export function LocalModelManager() {
  return (
    <div className="rounded-md border border-border-primary bg-background-secondary p-3 text-sm">
      <div className="font-medium text-text-primary mb-2">Local Models</div>
      <Button size="sm" variant="secondary">Manage</Button>
    </div>
  );
}
export default LocalModelManager;