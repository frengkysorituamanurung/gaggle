import React, { useState } from "react";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { api } from "../../lib/api";

export function ImportRecipeForm({ onImported }: { onImported?: () => void }) {
  const [text, setText] = useState("");
  const [name, setName] = useState("");
  const [title, setTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);

  const submit = async () => {
    if (!text.trim()) return;
    setImporting(true);
    setError(null);
    try {
      await api.recipe.create({
        name: name.trim() || "imported-recipe",
        title: title.trim() || "Imported Recipe",
        prompt: text,
      });
      setText("");
      setName("");
      setTitle("");
      onImported?.();
    } catch (e) {
      setError(String(e));
    } finally {
      setImporting(false);
    }
  };

  return (
    <div className="space-y-2">
      <Input placeholder="Name (optional)" value={name} onChange={(e) => setName(e.target.value)} />
      <Input placeholder="Title (optional)" value={title} onChange={(e) => setTitle(e.target.value)} />
      <textarea
        className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm min-h-[120px]"
        placeholder="Paste recipe YAML or prompt here"
        value={text}
        onChange={(e) => setText(e.target.value)}
      />
      {error && <div className="text-text-danger text-sm">{error}</div>}
      <div className="flex justify-end">
        <Button variant="default" size="sm" onClick={submit} disabled={importing || !text.trim()}>
          {importing ? "Importing…" : "Import"}
        </Button>
      </div>
    </div>
  );
}

export default ImportRecipeForm;