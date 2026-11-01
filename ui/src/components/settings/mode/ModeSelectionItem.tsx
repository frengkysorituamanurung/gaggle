import React from "react";

export function ModeSelectionItem({
  label,
  selected,
  onClick,
}: {
  label: string;
  selected?: boolean;
  onClick?: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`px-3 py-2 rounded-md border text-sm ${selected ? "border-text-primary bg-background-secondary text-text-primary" : "border-border-primary text-text-secondary"}`}
    >
      {label}
    </button>
  );
}

export default ModeSelectionItem;