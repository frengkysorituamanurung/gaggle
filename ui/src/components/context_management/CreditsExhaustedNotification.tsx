import React from "react";
import { Button } from "../ui/Button";

export function CreditsExhaustedNotification({ onAction }: { onAction?: () => void }) {
  return (
    <div className="rounded-md border border-text-danger bg-background-secondary px-3 py-2 text-sm">
      <div className="text-text-danger font-medium">Credits exhausted</div>
      <p className="text-xs text-text-secondary mt-1">You have run out of credits.</p>
      {onAction && (
        <Button size="sm" variant="default" className="mt-2" onClick={onAction}>Top up</Button>
      )}
    </div>
  );
}

export default CreditsExhaustedNotification;