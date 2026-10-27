import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";

export default function AgentLoopSettings() {
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Agent Loop</CardTitle></CardHeader>
      <CardContent>
        <p className="text-sm text-text-secondary">Max steps, compaction threshold, and tool concurrency are configured per chat. Global defaults coming soon.</p>
      </CardContent>
    </Card>
  );
}