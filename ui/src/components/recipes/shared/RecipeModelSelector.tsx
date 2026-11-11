import React from "react";
export function RecipeModelSelector({ value, onChange }: { value?: string; onChange?: (v: string) => void }) {
  return (
    <select className="px-2 py-1 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm" value={value || ""} onChange={(e) => onChange?.(e.target.value)}>
      <option value="">Default model</option>
    </select>
  );
}
export default RecipeModelSelector;