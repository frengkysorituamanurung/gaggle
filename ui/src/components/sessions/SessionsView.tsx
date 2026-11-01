import React, { useEffect, useState } from "react";
import { useApp } from "../../lib/store";
import { api } from "../../lib/api";
import type * as T from "../../types";
import { SessionListView } from "./SessionListView";
import { EmptyState } from "../EmptyState";
import { LoadingState } from "../LoadingState";
import { ErrorState } from "../ErrorState";

export function SessionsView() {
  const { projectId, setChatId } = useApp();
  const [chats, setChats] = useState<T.Chat[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = () => {
    if (!projectId) {
      setLoading(false);
      return;
    }
    setLoading(true);
    api.projectChats(projectId).then(setChats).catch((e) => setError(String(e))).finally(() => setLoading(false));
  };
  useEffect(refresh, [projectId]);

  if (loading) return <LoadingState />;
  if (error) return <ErrorState message={error} onRetry={refresh} />;
  if (chats.length === 0) return <EmptyState title="No sessions" description="Start a new chat to create a session." />;

  return (
    <SessionListView
      sessions={chats}
      onSelect={(c) => setChatId(c.id)}
      onRename={async (c) => {
        const title = prompt("New title", c.title);
        if (!title) return;
        await api.chats.update(c.id, { title });
        refresh();
      }}
    />
  );
}

export default SessionsView;