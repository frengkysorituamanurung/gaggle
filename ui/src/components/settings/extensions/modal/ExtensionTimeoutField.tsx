import React from "react";
import { Input } from "../../../ui/Input";
export function ExtensionTimeoutField({ value, onChange }: { value?: number; onChange?: (v: number) => void }) {
  return <Input type="number" placeholder="Timeout (secs)" value={value ?? ""} onChange={(e) => onChange?.(Number(e.target.value))} />;
}
export default ExtensionTimeoutField;