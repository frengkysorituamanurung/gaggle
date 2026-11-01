import React from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../../../ui/Dialog";
import { Button } from "../../../ui/Button";

export function SwitchModelModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  return (
    <Dialog open={open} onOpenChange={(v: boolean) => !v && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Switch Model</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-text-secondary">Select a model to switch to.</p>
        <div className="mt-3 flex justify-end">
          <Button size="sm" variant="secondary" onClick={onClose}>Close</Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default SwitchModelModal;