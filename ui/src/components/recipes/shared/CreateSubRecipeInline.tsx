import React, { useState } from "react";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";

export function CreateSubRecipeInline({ onCreate }: { onCreate?: (name: string) => void }) {
  const [name, setName] = useState("");
  return (
    <div className="flex items-center gap-2">
      <Input placeholder="Sub-recipe name" value={name} onChange={(e) => setName(e.target.value)} />
      <Button size="sm" variant="default" onClick={() => { if (name.trim()) { onCreate?.(name.trim()); setName(""); } }} disabled={!name.trim()}>
        Add
      </Button>
    </div>
  );
}

export default CreateSubRecipeInline;