import React from "react";
import { Button } from "../../ui/Button";

export function UpdateSection() {
  return (
    <div className="rounded-md border border-border-primary bg-background-secondary p-3 text-sm">
      <div className="font-medium text-text-primary mb-2">Updates</div>
      <p className="text-xs text-text-secondary mb-2">Check for application updates.</p>
      <Button size="sm" variant="secondary">Check for updates</Button>
    </div>
  );
}

export default UpdateSection;