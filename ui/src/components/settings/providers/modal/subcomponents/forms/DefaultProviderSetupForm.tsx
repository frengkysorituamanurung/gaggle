import React from "react";
import { Input } from "../../../../../ui/Input";
export function DefaultProviderSetupForm({ name, onNameChange }: { name?: string; onNameChange?: (v: string) => void }) {
  return <Input placeholder="Provider name" value={name || ""} onChange={(e: React.ChangeEvent<HTMLInputElement>) => onNameChange?.(e.target.value)} />;
}
export default DefaultProviderSetupForm;