import React from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../ui/Dialog";
import { Button } from "../ui/Button";

export function RecipeConsentModalContainer({
  open,
  onClose,
  onAccept,
  message,
}: {
  open: boolean;
  onClose: () => void;
  onAccept?: () => void;
  message?: string;
}) {
  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Consent Required</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-text-secondary">{message || "This recipe requires your consent to run."}</p>
        <div className="mt-3 flex justify-end gap-2">
          <Button size="sm" variant="secondary" onClick={onClose}>Decline</Button>
          <Button size="sm" variant="default" onClick={() => { onAccept?.(); onClose(); }}>Accept</Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default RecipeConsentModalContainer;