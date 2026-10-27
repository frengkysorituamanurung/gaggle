import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/Card";
import { Button } from "./ui/Button";
export default function ElicitationRequest({ message, onAccept, onDecline }: { message: string; onAccept: () => void; onDecline: () => void }) {
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Permission Request</CardTitle></CardHeader>
      <CardContent>
        <p className="text-sm text-text-secondary mb-3">{message}</p>
        <div className="flex gap-2">
          <Button size="sm" variant="default" onClick={onAccept}>Allow</Button>
          <Button size="sm" variant="secondary" onClick={onDecline}>Deny</Button>
        </div>
      </CardContent>
    </Card>
  );
}