import React from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "../ui/Dialog";
import { Button } from "../ui/Button";
export default function PrivacyInfoModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Privacy</DialogTitle>
          <DialogDescription>Gaggle runs locally. Your data stays on your machine unless you connect to a remote provider.</DialogDescription>
        </DialogHeader>
        <DialogFooter><Button variant="default" onClick={onClose}>OK</Button></DialogFooter>
      </DialogContent>
    </Dialog>
  );
}