import React from "react";
import { Input } from "../../../../../ui/Input";
export function ProviderCatalogPicker({ value, onChange }: { value?: string; onChange?: (v: string) => void }) {
  return <Input placeholder="Catalog model" value={value || ""} onChange={(e: React.ChangeEvent<HTMLInputElement>) => onChange?.(e.target.value)} />;
}
export default ProviderCatalogPicker;