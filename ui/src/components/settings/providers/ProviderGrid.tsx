import React from "react";
import type * as T from "../../../types";
import { ProviderCard } from "./ProviderCard";

export function ProviderGrid({
  providers = [],
  onSelect,
}: {
  providers?: T.ProviderStatus[];
  onSelect?: (p: T.ProviderStatus) => void;
}) {
  return (
    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
      {providers.map((p) => (
        <ProviderCard key={p.provider.id} provider={p} onClick={() => onSelect?.(p)} />
      ))}
    </div>
  );
}

export default ProviderGrid;