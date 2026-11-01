import React from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "../../../ui/Tooltip";
export function TooltipWrapper({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
export default TooltipWrapper;