import React from "react";
export default function MessageQueue({ count }: { count: number }) {
  if (count <= 0) return null;
  return <div className="text-xs text-text-secondary">{count} message(s) queued</div>;
}