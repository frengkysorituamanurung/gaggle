import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card";
import { Button } from "../ui/Button";
export default function ProviderSelector({ onSelect }: { onSelect: (kind: string) => void }) {
  const kinds = ["openai", "anthropic", "ollama", "openrouter", "deepseek", "xai", "google"];
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Select Provider</CardTitle></CardHeader>
      <CardContent>
        <div className="flex flex-wrap gap-2">
          {kinds.map((k) => (<Button key={k} size="sm" variant="secondary" onClick={() => onSelect(k)}>{k}</Button>))}
        </div>
      </CardContent>
    </Card>
  );
}