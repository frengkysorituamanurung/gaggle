import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card";
export default function ContextManagementPanel() {
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Context Management</CardTitle></CardHeader>
      <CardContent><p className="text-sm text-text-secondary">Compact threshold and context window settings. (Coming soon.)</p></CardContent>
    </Card>
  );
}