import React, { useState } from "react";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

export type HeaderEntry = { key: string; value: string };

export function HeadersSection({
  headers = [],
  onChange,
}: {
  headers?: HeaderEntry[];
  onChange?: (next: HeaderEntry[]) => void;
}) {
  const [list, setList] = useState<HeaderEntry[]>(headers);
  const update = (next: HeaderEntry[]) => {
    setList(next);
    onChange?.(next);
  };
  return (
    <div className="space-y-2">
      <div className="text-xs font-semibold text-text-primary">Headers</div>
      {list.map((h, i) => (
        <div key={i} className="flex gap-2">
          <Input placeholder="Key" value={h.key} onChange={(e) => {
            const next = [...list]; next[i] = { ...next[i], key: e.target.value }; update(next);
          }} />
          <Input placeholder="Value" value={h.value} onChange={(e) => {
            const next = [...list]; next[i] = { ...next[i], value: e.target.value }; update(next);
          }} />
          <Button size="sm" variant="destructive" onClick={() => update(list.filter((_, j) => j !== i))}>×</Button>
        </div>
      ))}
      <Button size="sm" variant="secondary" onClick={() => update([...list, { key: "", value: "" }])}>Add Header</Button>
    </div>
  );
}

export default HeadersSection;