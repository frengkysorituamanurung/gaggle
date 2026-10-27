import React, { useEffect, useState } from "react";
import { api } from "../../../lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";

export default function ResponseStylesSection() {
  const styles = ["default", "concise", "detailed", "formal", "casual"];
  const [selected, setSelected] = useState("default");

  useEffect(() => {
    api.settings.list().then((s) => {
      const v = s.find((x) => x.key === "response_style")?.value;
      if (v) setSelected(v);
    }).catch(() => {});
  }, []);

  const choose = async (s: string) => {
    setSelected(s);
    try { await api.settings.upsert("response_style", s); } catch { /* ignore */ }
  };

  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Response Styles</CardTitle></CardHeader>
      <CardContent>
        <p className="text-sm text-text-secondary mb-2">Preferred response style (applied per chat).</p>
        <div className="flex flex-wrap gap-2">
          {styles.map((s) => (
            <button
              key={s}
              onClick={() => choose(s)}
              className={`px-3 py-1.5 rounded-md border text-sm ${selected === s ? "border-border-primary bg-background-tertiary text-text-primary" : "border-border-primary text-text-primary hover:bg-background-secondary"}`}
            >
              {s}
            </button>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
