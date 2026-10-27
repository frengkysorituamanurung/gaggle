import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";
export default function LiveVoiceSettings() {
  return <Card><CardHeader><CardTitle className="text-text-primary text-base">Live Voice</CardTitle></CardHeader><CardContent><p className="text-sm text-text-secondary">Live voice requires a backend voice provider. (Coming soon.)</p></CardContent></Card>;
}