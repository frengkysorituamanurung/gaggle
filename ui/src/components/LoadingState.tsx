import React from "react";

export function LoadingState({ label = "Loading…" }: { label?: string }) {
  return (
    <div className="flex items-center justify-center py-8 text-text-secondary text-sm">
      <span className="animate-pulse">{label}</span>
    </div>
  );
}
