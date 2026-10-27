import React from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "./ui/Dialog";
import { Button } from "./ui/Button";
import { Input } from "./ui/Input";
export default function ParameterInputModal({ open, onClose, params, onSubmit }: { open: boolean; onClose: () => void; params: { name: string }[]; onSubmit: (vals: Record<string, string>) => void }) {
  const [vals, setVals] = React.useState<Record<string, string>>({});
  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent>
        <DialogHeader><DialogTitle>Recipe Parameters</DialogTitle></DialogHeader>
        <div className="flex flex-col gap-2">
          {params.map((p) => (<Input key={p.name} placeholder={p.name} value={vals[p.name] || ""} onChange={(e) => setVals((v) => ({ ...v, [p.name]: e.target.value }))} />))}
        </div>
        <DialogFooter>
          <Button variant="secondary" onClick={onClose}>Cancel</Button>
          <Button variant="default" onClick={() => { onSubmit(vals); onClose(); }}>Run</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}