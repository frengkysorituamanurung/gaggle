import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";
export default function DictationSettings() {
  return <Card><CardHeader><CardTitle className="text-text-primary text-base">Dictation</CardTitle></CardHeader><CardContent><p className="text-sm text-text-secondary">Dictation requires a speech-to-text backend. (Coming soon.)</p></CardContent></Card>;
}