import React from "react";

export function MessageUsageStats({
  inputTokens = 0,
  outputTokens = 0,
}: {
  inputTokens?: number;
  outputTokens?: number;
}) {
  return (
    <div className="text-xs text-text-tertiary">
      {inputTokens}↑ {outputTokens}↓ tokens
    </div>
  );
}

export default MessageUsageStats;