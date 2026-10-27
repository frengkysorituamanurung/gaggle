import React from "react";
import { Button } from "./ui/Button";
export default function ToolApprovalButtons({ onApprove, onDeny }: { onApprove: () => void; onDeny: () => void }) {
  return (
    <div className="flex gap-2">
      <Button size="sm" variant="default" onClick={onApprove}>Approve</Button>
      <Button size="sm" variant="destructive" onClick={onDeny}>Deny</Button>
    </div>
  );
}