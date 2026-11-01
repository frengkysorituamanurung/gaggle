import React from "react";
export function RecipeExtensionSelector({ selected = [], onChange }: { selected?: string[]; onChange?: (next: string[]) => void }) {
  return (
    <div className="text-sm text-text-secondary">
      Selected extensions: {selected.join(", ") || "none"}
    </div>
  );
}
export default RecipeExtensionSelector;