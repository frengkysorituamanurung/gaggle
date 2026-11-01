import React from "react";
import { Card, CardContent } from "./ui/Card";

export default function MentionPopover({
  items,
  onSelect,
}: {
  items: string[];
  onSelect?: (item: string) => void;
}) {
  if (!items.length) return null;
  return (
    <Card className="absolute z-50 mt-1 w-64 shadow-lg">
      <CardContent className="p-2 max-h-48 overflow-y-auto">
        {items.map((i) => (
          <div
            key={i}
            className="px-2 py-1 text-sm text-text-primary hover:bg-background-secondary rounded cursor-pointer"
            onClick={() => onSelect?.(i)}
          >
            {i}
          </div>
        ))}
      </CardContent>
    </Card>
  );
}
