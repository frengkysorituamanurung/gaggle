import React from "react";
import type { McpAppDisplayMode } from "./types";
export function useDisplayMode(initial: McpAppDisplayMode = "inline") {
  const [mode, setMode] = React.useState<McpAppDisplayMode>(initial);
  return { mode, setMode };
}