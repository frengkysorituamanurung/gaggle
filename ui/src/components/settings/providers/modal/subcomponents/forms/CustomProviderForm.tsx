import React from "react";
import { Input } from "../../../../../ui/Input";
export function CustomProviderForm({ baseUrl, onBaseUrlChange }: { baseUrl?: string; onBaseUrlChange?: (v: string) => void }) {
  return <Input placeholder="Base URL" value={baseUrl || ""} onChange={(e: React.ChangeEvent<HTMLInputElement>) => onBaseUrlChange?.(e.target.value)} />;
}
export default CustomProviderForm;