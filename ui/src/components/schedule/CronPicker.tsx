import React, { useState } from "react";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";

export function CronPicker({ value, onChange }: { value?: string; onChange?: (v: string) => void }) {
  const [expr, setExpr] = useState(value || "0 * * * *");
  return (
    <div className="flex items-center gap-2">
      <Input value={expr} onChange={(e) => { setExpr(e.target.value); onChange?.(e.target.value); }} placeholder="Cron expression" />
      <Button size="sm" variant="secondary" onClick={() => onChange?.(expr)}>Set</Button>
    </div>
  );
}

export default CronPicker;