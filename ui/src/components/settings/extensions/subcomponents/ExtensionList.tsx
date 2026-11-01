import React from "react";
import type * as T from "../../../../types";
import { ExtensionItem } from "./ExtensionItem";
export function ExtensionList({ extensions = [], onToggle }: { extensions?: T.Extension[]; onToggle?: (id: string, enabled: boolean) => void }) {
  return (
    <div className="space-y-1">
      {extensions.map((e) => <ExtensionItem key={e.id} ext={e} onToggle={onToggle} />)}
    </div>
  );
}
export default ExtensionList;