import React from "react";
import { ExtensionMenu } from "./ExtensionMenu";
import type * as T from "../../types";

export function BottomMenuExtensionSelection({
  extensions = [],
  onToggle,
}: {
  extensions?: T.Extension[];
  onToggle?: (id: string, enabled: boolean) => void;
}) {
  return <ExtensionMenu extensions={extensions} onToggle={onToggle} />;
}

export default BottomMenuExtensionSelection;