import React, { useEffect, useState } from "react";
import { api } from "../../../lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";
import { Switch } from "../../ui/Switch";
import { Button } from "../../ui/Button";

export default function AppSettingsSection() {
  const [settings, setSettings] = useState<Record<string, string>>({});
  useEffect(() => { api.settings.list().then((s) => { const m: Record<string, string> = {}; s.forEach((x) => (m[x.key] = x.value)); setSettings(m); }).catch(() => {}); }, []);
  const toggle = async (key: string) => {
    const cur = settings[key] === "true";
    try { await api.settings.upsert(key, !cur); setSettings((p) => ({ ...p, [key]: String(!cur) })); } catch { /* ignore */ }
  };
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">App Settings</CardTitle></CardHeader>
      <CardContent>
        <div className="flex items-center justify-between py-2"><span className="text-sm text-text-primary">Notifications</span><Switch checked={settings["notifications"] === "true"} onCheckedChange={() => toggle("notifications")} /></div>
        <div className="flex items-center justify-between py-2"><span className="text-sm text-text-primary">Prevent sleep during runs</span><Switch checked={settings["prevent_sleep"] === "true"} onCheckedChange={() => toggle("prevent_sleep")} /></div>
        <div className="flex items-center justify-between py-2"><span className="text-sm text-text-primary">Cost tracking</span><Switch checked={settings["cost_tracking"] === "true"} onCheckedChange={() => toggle("cost_tracking")} /></div>
        <div className="flex items-center justify-between py-2"><span className="text-sm text-text-primary">Telemetry</span><Switch checked={settings["telemetry_enabled"] === "true"} onCheckedChange={() => toggle("telemetry_enabled")} /></div>
        <div className="mt-3 text-xs text-text-secondary">Theme and language are managed in the header. Help & feedback: https://github.com. Version/updates: managed by the app shell.</div>
      </CardContent>
    </Card>
  );
}