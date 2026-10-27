import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";

export default function ExternalBackendSection() {
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">External Backend</CardTitle></CardHeader>
      <CardContent>
        <p className="text-sm text-text-secondary">Connect to a remote Gaggle backend instance. (Coming soon.)</p>
      </CardContent>
    </Card>
  );
}