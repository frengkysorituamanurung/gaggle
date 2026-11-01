import React from "react";

export function ItemIcon({ label }: { label?: string }) {
  return (
    <span className="inline-flex h-5 w-5 items-center justify-center rounded bg-background-secondary text-xs text-text-primary">
      {label?.[0] || "?"}
    </span>
  );
}

export default ItemIcon;