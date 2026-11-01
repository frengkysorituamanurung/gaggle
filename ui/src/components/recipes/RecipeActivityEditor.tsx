import React, { useState } from "react";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

export function RecipeActivityEditor({
  initial,
  onSave,
  onCancel,
}: {
  initial?: { name: string; description?: string };
  onSave: (data: { name: string; description: string }) => void;
  onCancel?: () => void;
}) {
  const [name, setName] = useState(initial?.name || "");
  const [description, setDescription] = useState(initial?.description || "");
  return (
    <div className="space-y-2">
      <Input placeholder="Activity name" value={name} onChange={(e) => setName(e.target.value)} />
      <Input placeholder="Description" value={description} onChange={(e) => setDescription(e.target.value)} />
      <div className="flex gap-2">
        <Button size="sm" variant="default" onClick={() => onSave({ name, description })} disabled={!name.trim()}>Save</Button>
        {onCancel && <Button size="sm" variant="secondary" onClick={onCancel}>Cancel</Button>}
      </div>
    </div>
  );
}

export default RecipeActivityEditor;