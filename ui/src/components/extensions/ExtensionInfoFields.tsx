import React from "react";
import { Input } from "../ui/Input";
export function ExtensionInfoFields({ description, onDescriptionChange }: { description?: string; onDescriptionChange?: (v: string) => void }) {
  return <Input placeholder="Description" value={description || ""} onChange={(e) => onDescriptionChange?.(e.target.value)} />;
}
export default ExtensionInfoFields;