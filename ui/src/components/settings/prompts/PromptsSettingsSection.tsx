import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";

export default function PromptsSettingsSection() {
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Prompts</CardTitle></CardHeader>
      <CardContent>
        <p className="text-sm text-text-secondary">Manage system prompt templates and overrides. (Coming soon.)</p>
      </CardContent>
    </Card>
  );
}