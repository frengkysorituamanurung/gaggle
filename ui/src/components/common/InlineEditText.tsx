import React, { useState } from "react";
import { Input } from "../ui/Input";

export function InlineEditText({
  value,
  onSave,
}: {
  value: string;
  onSave?: (v: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState(value);
  if (!editing) {
    return (
      <span className="cursor-pointer text-text-primary" onClick={() => setEditing(true)}>
        {value}
      </span>
    );
  }
  return (
    <Input
      autoFocus
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={() => { onSave?.(text); setEditing(false); }}
      onKeyDown={(e) => { if (e.key === "Enter") { onSave?.(text); setEditing(false); } }}
    />
  );
}

export default InlineEditText;