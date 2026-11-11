import React from "react";

export function EnvironmentBadge({ env }: { env?: string }) {
  if (!env) return null;
  return <span className="text-xs px-2 py-0.5 rounded-full bg-surface-2 text-text-secondary">{env}</span>;
}

export default EnvironmentBadge;