import React, { useEffect, useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "./ui/Dialog";
import { Button } from "./ui/Button";
import { api } from "../lib/api";

export default function TelemetryConsentPrompt() {
  const [open, setOpen] = useState(false);
  useEffect(() => {
    const seen = localStorage.getItem("gaggle.telemetry.seen");
    if (!seen) setOpen(true);
  }, []);
  const respond = async (enabled: boolean) => {
    try { await api.settings.upsert("telemetry_enabled", enabled); } catch { /* ignore */ }
    localStorage.setItem("gaggle.telemetry.seen", "1");
    setOpen(false);
  };
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Telemetry</DialogTitle>
          <DialogDescription>Help improve Gaggle by sharing anonymous usage data? You can change this later in Settings.</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="secondary" onClick={() => respond(false)}>No thanks</Button>
          <Button variant="default" onClick={() => respond(true)}>Allow</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}