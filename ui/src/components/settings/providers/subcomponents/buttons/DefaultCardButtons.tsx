import React from "react";
import { Button } from "../../../ui/Button";
export function DefaultCardButtons({ onEdit, onDelete }: { onEdit?: () => void; onDelete?: () => void }) {
  return (
    <div className="flex gap-2">
      {onEdit && <Button size="sm" variant="secondary" onClick={onEdit}>Edit</Button>}
      {onDelete && <Button size="sm" variant="destructive" onClick={onDelete}>Delete</Button>}
    </div>
  );
}
export default DefaultCardButtons;