import React from "react";

export function SystemNotificationInline({ message }: { message: string }) {
  return (
    <div className="rounded-md border border-border-primary bg-background-secondary px-3 py-2 text-sm text-text-secondary">
      {message}
    </div>
  );
}

export default SystemNotificationInline;