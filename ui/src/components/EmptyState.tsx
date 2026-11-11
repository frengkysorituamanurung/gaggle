import React from "react";
import { cn } from "../lib/utils";

export function EmptyState({ title, description, action }: { title: string; description?: string; action?: React.ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-center text-text-secondary">
      <div className="text-lg font-semibold text-text-primary mb-1">{title}</div>
      {description && <div className="text-sm mb-4">{description}</div>}
      {action}
    </div>
  );
}
