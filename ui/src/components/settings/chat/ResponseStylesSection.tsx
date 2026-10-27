import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";

export default function ResponseStylesSection() {
  const styles = ["default", "concise", "detailed", "formal", "casual"];
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Response Styles</CardTitle></CardHeader>
      <CardContent>
        <p className="text-sm text-text-secondary mb-2">Preferred response style (applied per chat).</p>
        <div className="flex flex-wrap gap-2">
          {styles.map((s) => (
            <button key={s} className="px-3 py-1.5 rounded-md border border-border-primary text-sm text-text-primary hover:bg-background-secondary">{s}</button>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}