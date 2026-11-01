import React, { useState } from "react";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

export type EnvVarEntry = { key: string; value: string };

export function EnvVarsSection({
  envVars = [],
  onChange,
}: {
  envVars?: EnvVarEntry[];
  onChange?: (next: EnvVarEntry[]) => void;
}) {
  const [list, setList] = useState<EnvVarEntry[]>(envVars);
  const update = (next: EnvVarEntry[]) => {
    setList(next);
    onChange?.(next);
  };
  return (
    <div className="space-y-2">
      <div className="text-xs font-semibold text-text-primary">Environment Variables</div>
      {list.map((e, i) => (
        <div key={i} className="flex gap-2">
          <Input placeholder="KEY" value={e.key} onChange={(ev) => {
            const next = [...list]; next[i] = { ...next[i], key: ev.target.value }; update(next);
          }} />
          <Input placeholder="Value" value={e.value} onChange={(ev) => {
            const next = [...list]; next[i] = { ...next[i], value: ev.target.value }; update(next);
          }} />
          <Button size="sm" variant="destructive" onClick={() => update(list.filter((_, j) => j !== i))}>×</Button>
        </div>
      ))}
      <Button size="sm" variant="secondary" onClick={() => update([...list, { key: "", value: "" }])}>Add Variable</Button>
    </div>
  );
}

export default EnvVarsSection;