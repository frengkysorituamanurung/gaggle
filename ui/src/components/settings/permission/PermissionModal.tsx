import React from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../../ui/Dialog";
import { Button } from "../../ui/Button";

export function PermissionModal({
  open,
  onClose,
  onConfirm,
  title,
  message,
}: {
  open: boolean;
  onClose: () => void;
  onConfirm?: () => void;
  title?: string;
  message?: string;
}) {
  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title || "Permission"}</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-text-secondary">{message || "Allow this action?"}</p>
        <div className="mt-3 flex justify-end gap-2">
          <Button size="sm" variant="secondary" onClick={onClose}>Deny</Button>
          <Button size="sm" variant="default" onClick={() => { onConfirm?.(); onClose(); }}>Allow</Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default PermissionModal;