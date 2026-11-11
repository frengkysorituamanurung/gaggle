import React from "react";

export function GroupedExtensionLoadingToast({ count = 0 }: { count?: number }) {
  if (count <= 0) return null;
  return (
    <div className="rounded-md border border-border-primary bg-background-secondary px-3 py-2 text-sm text-text-secondary">
      Loading {count} extension{count > 1 ? "s" : ""}…
    </div>
  );
}

export default GroupedExtensionLoadingToast;