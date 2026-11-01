import React, { useState } from "react";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";

export function LocalInferenceSettings() {
  const [baseUrl, setBaseUrl] = useState("http://localhost:11434");
  return (
    <div className="space-y-2 rounded-md border border-border-primary bg-background-secondary p-3 text-sm">
      <div className="font-medium text-text-primary">Local Inference</div>
      <label className="text-xs text-text-secondary">Base URL</label>
      <Input value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} />
      <Button size="sm" variant="secondary">Save</Button>
    </div>
  );
}

export default LocalInferenceSettings;