import React from "react";
import { Input } from "../../../ui/Input";
export function ExtensionConfigFields({ name, onNameChange }: { name?: string; onNameChange?: (v: string) => void }) {
  return <Input placeholder="Name" value={name || ""} onChange={(e) => onNameChange?.(e.target.value)} />;
}
export default ExtensionConfigFields;