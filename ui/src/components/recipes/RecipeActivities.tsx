import React from "react";

export type Activity = { id: string; name: string; description?: string };

export function RecipeActivities({
  activities = [],
  onEdit,
  onRemove,
}: {
  activities?: Activity[];
  onEdit?: (a: Activity) => void;
  onRemove?: (id: string) => void;
}) {
  if (activities.length === 0) {
    return <div className="text-sm text-text-secondary">No activities defined.</div>;
  }
  return (
    <div className="space-y-2">
      {activities.map((a) => (
        <div key={a.id} className="flex items-center justify-between rounded-md border border-border-primary bg-background-secondary p-2">
          <div>
            <div className="text-sm font-medium text-text-primary">{a.name}</div>
            {a.description && <div className="text-xs text-text-secondary">{a.description}</div>}
          </div>
          <div className="flex gap-2">
            {onEdit && <button className="text-xs text-text-primary underline" onClick={() => onEdit(a)}>Edit</button>}
            {onRemove && <button className="text-xs text-text-danger underline" onClick={() => onRemove(a.id)}>Remove</button>}
          </div>
        </div>
      ))}
    </div>
  );
}

export default RecipeActivities;