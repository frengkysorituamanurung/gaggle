import React, { useState } from "react";
import { Switch } from "../../ui/Switch";
import { Input } from "../../ui/Input";

export function DictationSettings() {
  const [enabled, setEnabled] = useState(false);
  const [language, setLanguage] = useState("en");
  return (
    <div className="space-y-3 rounded-md border border-border-primary bg-background-secondary p-3 text-sm">
      <div className="flex items-center justify-between">
        <div className="font-medium text-text-primary">Dictation</div>
        <Switch checked={enabled} onCheckedChange={setEnabled} />
      </div>
      <div>
        <label className="text-xs text-text-secondary">Language</label>
        <Input value={language} onChange={(e) => setLanguage(e.target.value)} />
      </div>
    </div>
  );
}

export default DictationSettings;