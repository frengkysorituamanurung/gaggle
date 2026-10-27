import React from "react";
export default function SubagentSessionNotice({ name }: { name: string }) {
  return <div className="text-xs text-text-tertiary italic">Sub-agent session: {name}</div>;
}