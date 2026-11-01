import { useState, useEffect } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";

export function useNavigationSessions() {
  const { projectId } = useApp();
  const [sessions, setSessions] = useState<T.Chat[]>([]);
  const [loading, setLoading] = useState(false);
  useEffect(() => {
    if (!projectId) { setSessions([]); return; }
    setLoading(true);
    api.projectChats(projectId).then(setSessions).catch(() => {}).finally(() => setLoading(false));
  }, [projectId]);
  return { sessions, loading };
}

export default useNavigationSessions;