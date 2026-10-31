import React, { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card";
import { Button } from "../ui/Button";
import { Switch } from "../ui/Switch";

export default function DictationSettingsSection() {
  const [enabled, setEnabled] = useState(false);
  const [language, setLanguage] = useState("en");

  const LANGUAGES = [
    { id: "en", label: "English" },
    { id: "es", label: "Spanish" },
    { id: "fr", label: "French" },
    { id: "de", label: "German" },
    { id: "ja", label: "Japanese" },
    { id: "zh", label: "Chinese" },
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-text-primary text-base">Dictation</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-center justify-between mb-3">
          <span className="text-sm text-text-primary">Enable dictation</span>
          <Switch checked={enabled} onCheckedChange={setEnabled} />
        </div>
        {enabled && (
          <div className="flex flex-col gap-2">
            <label className="text-xs text-text-secondary">Language</label>
            <select
              className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
            >
              {LANGUAGES.map((l) => (
                <option key={l.id} value={l.id}>{l.label}</option>
              ))}
            </select>
            <p className="text-text-secondary text-xs">
              Dictation uses system speech recognition. Requires microphone access.
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}