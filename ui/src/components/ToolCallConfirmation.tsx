import React from "react";
import { Button } from "./ui/Button";
export default function ToolCallConfirmation({ toolName, onConfirm, onCancel }: { toolName: string; onConfirm: () => void; onCancel: () => void }) {
  return (
    <div className="flex items-center gap-2 p-2 border border-border-primary rounded-md bg-background-secondary">
      <span className="text-sm text-text-primary">Run {toolName}?</span>
      <Button size="sm" variant="default" onClick={onConfirm}>Yes</Button>
      <Button size="sm" variant="secondary" onClick={onCancel}>No</Button>
    </div>
  );
}