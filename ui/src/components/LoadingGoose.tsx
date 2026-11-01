import React from "react";

export function LoadingGoose({ label = "Loading…" }: { label?: string }) {
  return (
    <div className="flex items-center justify-center gap-2 text-text-secondary text-sm" role="status" aria-live="polite">
      <span className="animate-spin inline-block h-4 w-4 border-2 border-text-secondary border-t-transparent rounded-full motion-reduce:animate-none" />
      <span>{label}</span>
    </div>
  );
}

export default LoadingGoose;