import React from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../../../ui/Dialog";
import { Button } from "../../../ui/Button";
export function ExtensionModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Extension</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-text-secondary">Configure extension here.</p>
        <div className="mt-3 flex justify-end">
          <Button size="sm" variant="secondary" onClick={onClose}>Close</Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
export default ExtensionModal;