import React from "react";
import { Input } from "../../ui/Input";
export function SubRecipeEditor({ name, onNameChange }: { name?: string; onNameChange?: (v: string) => void }) {
  return <Input placeholder="Sub-recipe name" value={name || ""} onChange={(e) => onNameChange?.(e.target.value)} />;
}
export default SubRecipeEditor;