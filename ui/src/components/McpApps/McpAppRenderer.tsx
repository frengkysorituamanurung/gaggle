import React from "react";
import type { McpAppProps } from "./types";
export default function McpAppRenderer({ url, toolName }: McpAppProps) {
  return (
    <div className="border border-border-primary rounded-xl p-3 bg-background-secondary">
      <div className="text-sm text-text-primary font-medium">{toolName || "MCP App"}</div>
      {url ? <iframe src={url} className="w-full h-64 mt-2 border-0 rounded-md" title={toolName} /> : <div className="text-xs text-text-secondary mt-2">No app URL.</div>}
    </div>
  );
}