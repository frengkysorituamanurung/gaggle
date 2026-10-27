import React from "react";
import { cn } from "../../lib/utils";

export function MainPanelLayout({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={cn("flex flex-col h-full w-full", className)}>{children}</div>;
}