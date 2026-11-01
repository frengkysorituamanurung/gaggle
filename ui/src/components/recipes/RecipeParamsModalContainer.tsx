import React, { useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

export function RecipeParamsModalContainer({
  open,
  onClose,
  params = [],
  onSubmit,
}: {
  open: boolean;
  onClose: () => void;
  params?: string[];
  onSubmit?: (values: Record<string, string>) => void;
}) {
  const [values, setValues] = useState<Record<string, string>>({});
  React.useEffect(() => {
    if (open) setValues({});
  }, [open]);
  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Recipe Parameters</DialogTitle>
        </DialogHeader>
        <div className="space-y-2">
          {params.length === 0 && <div className="text-sm text-text-secondary">No parameters.</div>}
          {params.map((p) => (
            <div key={p}>
              <label className="text-xs text-text-secondary">{p}</label>
              <Input value={values[p] || ""} onChange={(e) => setValues((v) => ({ ...v, [p]: e.target.value }))} />
            </div>
          ))}
          <div className="flex justify-end gap-2">
            <Button size="sm" variant="secondary" onClick={onClose}>Cancel</Button>
            <Button size="sm" variant="default" onClick={() => { onSubmit?.(values); onClose(); }}>Run</Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default RecipeParamsModalContainer;