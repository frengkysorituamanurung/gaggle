import React, { useState } from "react";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
export function KeyValueEditor({ items = [], onChange }: { items?: { key: string; value: string }[]; onChange?: (next: { key: string; value: string }[]) => void }) {
  const [list, setList] = useState(items);
  const update = (next: { key: string; value: string }[]) => { setList(next); onChange?.(next); };
  return (
    <div className="space-y-2">
      {list.map((kv, i) => (
        <div key={i} className="flex gap-2">
          <Input placeholder="Key" value={kv.key} onChange={(e) => { const n = [...list]; n[i] = { ...n[i], key: e.target.value }; update(n); }} />
          <Input placeholder="Value" value={kv.value} onChange={(e) => { const n = [...list]; n[i] = { ...n[i], value: e.target.value }; update(n); }} />
          <Button size="sm" variant="destructive" onClick={() => update(list.filter((_, j) => j !== i))}>×</Button>
        </div>
      ))}
      <Button size="sm" variant="secondary" onClick={() => update([...list, { key: "", value: "" }])}>Add</Button>
    </div>
  );
}
export default KeyValueEditor;