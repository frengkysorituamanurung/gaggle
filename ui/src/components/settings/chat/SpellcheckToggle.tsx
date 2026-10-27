import React from "react";
import { useApp } from "../../../lib/store";
import { api } from "../../../lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";
import { Switch } from "../../ui/Switch";

export default function SpellcheckToggle() {
  const [enabled, setEnabled] = React.useState(true);
  React.useEffect(() => {
    api.settings.list().then((s) => {
      const v = s.find((x) => x.key === "spellcheck")?.value;
      if (v !== undefined) setEnabled(v === "true");
    }).catch(() => {});
  }, []);
  const toggle = async () => {
    const next = !enabled;
    setEnabled(next);
    try { await api.settings.upsert("spellcheck", next); } catch { /* ignore */ }
  };
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Spellcheck</CardTitle></CardHeader>
      <CardContent>
        <div className="flex items-center justify-between py-2">
          <span className="text-sm text-text-primary">Enable spellcheck in composer</span>
          <Switch checked={enabled} onCheckedChange={toggle} />
        </div>
      </CardContent>
    </Card>
  );
}