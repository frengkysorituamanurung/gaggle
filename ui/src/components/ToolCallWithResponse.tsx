import React from "react";
import { ToolCallCard } from "./ToolCallCard";

export function ToolCallWithResponse({ label, request, response }: { label: string; request?: string; response?: string }) {
  return (
    <div className="space-y-1">
      <ToolCallCard label={label} content={request} />
      {response && <ToolCallCard label={`${label} result`} content={response} />}
    </div>
  );
}

export default ToolCallWithResponse;