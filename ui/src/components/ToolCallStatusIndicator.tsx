import React from "react";

export function ToolCallStatusIndicator({ status }: { status?: string }) {
  const color = status === "completed" || status === "ok" ? "text-text-success" : status === "failed" || status === "error" ? "text-text-danger" : "text-text-secondary";
  return <span className={`text-xs ${color}`}>{status || "pending"}</span>;
}

export default ToolCallStatusIndicator;