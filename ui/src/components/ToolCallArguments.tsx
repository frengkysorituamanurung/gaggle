import React from "react";

export function ToolCallArguments({ args }: { args?: string }) {
  if (!args) return null;
  return <pre className="text-xs text-text-secondary bg-background-primary p-2 rounded border border-border-primary overflow-x-auto">{args}</pre>;
}

export default ToolCallArguments;