import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";
import { Switch } from "../../ui/Switch";

export default function SecurityToggle() {
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Security</CardTitle></CardHeader>
      <CardContent>
        <div className="flex items-center justify-between py-2">
          <span className="text-sm text-text-primary">Require approval for shell commands</span>
          <Switch defaultChecked />
        </div>
        <div className="flex items-center justify-between py-2">
          <span className="text-sm text-text-primary">Restrict filesystem to project root</span>
          <Switch defaultChecked />
        </div>
      </CardContent>
    </Card>
  );
}