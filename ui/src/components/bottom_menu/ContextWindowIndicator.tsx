import React from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/Tooltip";

export function ContextWindowIndicator({
  totalTokens = 0,
  contextLimit = 128000,
}: {
  totalTokens?: number;
  contextLimit?: number;
}) {
  const pct = contextLimit ? Math.min(100, Math.round((totalTokens / contextLimit) * 100)) : 0;
  const color = pct > 90 ? "text-text-danger" : pct > 75 ? "text-text-warning" : "text-text-secondary";
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div className={`flex items-center gap-1 text-xs font-mono ${color}`}>
          <span>{totalTokens.toLocaleString()}</span>
          <span className="text-text-tertiary">/</span>
          <span className="text-text-tertiary">{contextLimit.toLocaleString()}</span>
          <span className="text-text-tertiary">({pct}%)</span>
        </div>
      </TooltipTrigger>
      <TooltipContent>Context window usage</TooltipContent>
    </Tooltip>
  );
}

export default ContextWindowIndicator;