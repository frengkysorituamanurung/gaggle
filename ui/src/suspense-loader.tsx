import React from "react";
export function SuspenseLoader({ children }: { children: React.ReactNode }) {
  return <React.Suspense fallback={<div className="p-4 text-text-secondary text-sm">Loading…</div>}>{children}</React.Suspense>;
}
export default SuspenseLoader;