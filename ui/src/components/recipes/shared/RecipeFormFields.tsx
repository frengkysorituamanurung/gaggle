import React from "react";
import { Input } from "../../ui/Input";
export function RecipeFormFields({ name, onNameChange, title, onTitleChange }: { name?: string; onNameChange?: (v: string) => void; title?: string; onTitleChange?: (v: string) => void }) {
  return (
    <div className="space-y-2">
      <Input placeholder="Name" value={name || ""} onChange={(e) => onNameChange?.(e.target.value)} />
      <Input placeholder="Title" value={title || ""} onChange={(e) => onTitleChange?.(e.target.value)} />
    </div>
  );
}
export default RecipeFormFields;