import React, { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card";
import { Button } from "../ui/Button";
import { Switch } from "../ui/Switch";

export default function LiveVoiceSettingsSection() {
  const [enabled, setEnabled] = useState(false);
  const [apiKey, setApiKey] = useState("");

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-text-primary text-base">Live Voice</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-center justify-between mb-3">
          <span className="text-sm text-text-primary">Enable live voice interaction</span>
          <Switch checked={enabled} onCheckedChange={setEnabled} />
        </div>
        {enabled && (
          <div className="flex flex-col gap-2">
            <label className="text-xs text-text-secondary">OpenAI API Key (for Whisper)</label>
            <input
              type="password"
              className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="sk-..."
            />
            <p className="text-text-secondary text-xs">
              Live voice requires an OpenAI API key for transcription.
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}