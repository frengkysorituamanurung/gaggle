import { useState, useEffect } from "react";
import { api } from "../lib/api";
import type * as T from "../types";

export function useChatSession(chatId: string | null) {
  const [session, setSession] = useState<T.Chat | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!chatId) { setSession(null); return; }
    setLoading(true);
    api.chats.get(chatId).then(setSession).catch((e) => setError(String(e))).finally(() => setLoading(false));
  }, [chatId]);
  return { session, loading, error };
}

export default useChatSession;