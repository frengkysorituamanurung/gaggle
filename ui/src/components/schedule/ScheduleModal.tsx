import React, { useState, useEffect } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { CronPicker } from "./CronPicker";
import { api } from "../../lib/api";

export function ScheduleModal({
  open,
  onClose,
  onSaved,
  projectId,
}: {
  open: boolean;
  onClose: () => void;
  onSaved?: () => void;
  projectId?: string | null;
}) {
  const [recipeId, setRecipeId] = useState("");
  const [cron, setCron] = useState("0 * * * *");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setRecipeId("");
      setCron("0 * * * *");
      setError(null);
    }
  }, [open]);

  const save = async () => {
    if (!recipeId.trim()) return;
    setSaving(true);
    setError(null);
    try {
      await api.schedule.create({ recipe_id: recipeId.trim(), project_id: projectId || null, cron_expr: cron });
      onSaved?.();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create Schedule</DialogTitle>
        </DialogHeader>
        <div className="space-y-2">
          <Input placeholder="Recipe ID" value={recipeId} onChange={(e) => setRecipeId(e.target.value)} />
          <CronPicker value={cron} onChange={setCron} />
          {error && <div className="text-text-danger text-sm">{error}</div>}
          <div className="flex justify-end gap-2">
            <Button variant="secondary" size="sm" onClick={onClose}>Cancel</Button>
            <Button variant="default" size="sm" onClick={save} disabled={saving || !recipeId.trim()}>
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default ScheduleModal;