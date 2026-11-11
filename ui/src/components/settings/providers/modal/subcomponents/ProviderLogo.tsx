import React from "react";
export function ProviderLogo({ name }: { name?: string }) {
  return <span className="inline-flex h-8 w-8 items-center justify-center rounded bg-background-secondary text-text-primary text-xs">{(name || "?")[0]}</span>;
}
export default ProviderLogo;