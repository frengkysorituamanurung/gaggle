import { useState, useCallback } from "react";

export type Alert = { id: string; message: string; type?: "info" | "warning" | "error" | "success" };

export function useAlerts() {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const add = useCallback((a: Alert) => setAlerts((prev) => [...prev, a]), []);
  const remove = useCallback((id: string) => setAlerts((prev) => prev.filter((a) => a.id !== id)), []);
  const clear = useCallback(() => setAlerts([]), []);
  return { alerts, add, remove, clear };
}

export default useAlerts;