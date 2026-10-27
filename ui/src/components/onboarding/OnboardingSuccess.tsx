import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card";
import { Button } from "../ui/Button";
export default function OnboardingSuccess({ onDone }: { onDone: () => void }) {
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">You're all set</CardTitle></CardHeader>
      <CardContent>
        <p className="text-sm text-text-secondary mb-3">Provider configured. Start chatting.</p>
        <Button variant="default" onClick={onDone}>Continue</Button>
      </CardContent>
    </Card>
  );
}