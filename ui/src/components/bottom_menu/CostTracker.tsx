import React from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/Tooltip";
import { defineMessages, useIntl } from "../../i18n";

const messages = defineMessages({
  totalSessionCost: { id: "costTracker.totalSessionCost", defaultMessage: "Total session cost: {cost}" },
  inputOutput: { id: "costTracker.inputOutput", defaultMessage: "Input: {input} tokens | Output: {output} tokens" },
});

export function CostTracker({
  inputTokens = 0,
  outputTokens = 0,
  accumulatedCost = null,
  model = null,
  provider = null,
}: {
  inputTokens?: number;
  outputTokens?: number;
  accumulatedCost?: number | null;
  model?: string | null;
  provider?: string | null;
}) {
  const intl = useIntl();
  if (!model || !provider) return null;
  const totalCost = accumulatedCost ?? 0;
  const currency = "$";
  const tooltip = accumulatedCost != null
    ? intl.formatMessage(messages.totalSessionCost, { cost: `${currency}${totalCost.toFixed(4)}` }) +
      "\n" +
      intl.formatMessage(messages.inputOutput, {
        input: inputTokens.toLocaleString(),
        output: outputTokens.toLocaleString(),
      })
    : intl.formatMessage(messages.inputOutput, {
        input: inputTokens.toLocaleString(),
        output: outputTokens.toLocaleString(),
      });
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div className="flex items-center justify-center h-full transition-colors cursor-default translate-y-[1px] text-text-primary/70 hover:text-text-primary">
          <span className="text-xs font-mono">
            {currency}
            {totalCost.toFixed(2)}
          </span>
        </div>
      </TooltipTrigger>
      <TooltipContent>{tooltip}</TooltipContent>
    </Tooltip>
  );
}

export default CostTracker;