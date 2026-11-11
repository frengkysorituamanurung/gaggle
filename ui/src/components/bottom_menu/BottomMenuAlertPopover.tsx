import React, { useState } from "react";
import { Button } from "../ui/Button";
import { Bell } from "lucide-react";

export type AlertItem = { id: string; message: string; level?: "info" | "warning" | "error" };

export function BottomMenuAlertPopover({ alerts = [] }: { alerts?: AlertItem[] }) {
  const [open, setOpen] = useState(false);
  const hasAlerts = alerts.length > 0;
  return (
    <div className="relative">
      <Button variant="ghost" size="sm" onClick={() => setOpen((v) => !v)} title="Alerts">
        <Bell className={`w-4 h-4 ${hasAlerts ? "text-text-danger" : ""}`} />
        {hasAlerts && (
          <span className="ml-1 text-xs text-text-danger">{alerts.length}</span>
        )}
      </Button>
      {open && (
        <div className="absolute bottom-full mb-2 z-50 w-72 max-h-64 overflow-y-auto rounded-md border border-border-primary bg-background-primary p-2 shadow-lg">
          {!hasAlerts && <div className="text-xs text-text-secondary p-2">No alerts.</div>}
          {alerts.map((a) => (
            <div key={a.id} className="px-2 py-1.5 text-sm text-text-primary border-b border-border-primary last:border-0">
              <span className={`text-xs font-semibold ${a.level === "error" ? "text-text-danger" : a.level === "warning" ? "text-text-warning" : "text-text-secondary"}`}>
                {a.level || "info"}
              </span>
              <div className="text-xs text-text-secondary mt-0.5">{a.message}</div>
            </div>
          ))}
          <div className="mt-2 flex justify-end">
            <Button size="sm" variant="secondary" onClick={() => setOpen(false)}>Close</Button>
          </div>
        </div>
      )}
    </div>
  );
}

export default BottomMenuAlertPopover;