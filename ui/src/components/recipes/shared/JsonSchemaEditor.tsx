import React, { useState } from "react";
export function JsonSchemaEditor({ value, onChange }: { value?: string; onChange?: (v: string) => void }) {
  const [text, setText] = useState(value || "{}");
  return (
    <textarea
      className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm min-h-[100px] font-mono"
      value={text}
      onChange={(e) => { setText(e.target.value); onChange?.(e.target.value); }}
    />
  );
}
export default JsonSchemaEditor;