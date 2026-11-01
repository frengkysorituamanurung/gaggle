import React, { createContext, useContext, useState } from "react";
import { useApp } from "../lib/store";

type ChatSessionsContextValue = {
  openSessions: string[];
  activeSessionId: string | null;
  openSession: (id: string) => void;
  closeSession: (id: string) => void;
  setActiveSession: (id: string | null) => void;
};

const ChatSessionsContext = createContext<ChatSessionsContextValue | null>(null);

export function useChatSessions() {
  const ctx = useContext(ChatSessionsContext);
  if (!ctx) throw new Error("useChatSessions must be used within ChatSessionsContainer");
  return ctx;
}

export function ChatSessionsContainer({ children }: { children: React.ReactNode }) {
  const { chatId } = useApp();
  const [openSessions, setOpenSessions] = useState<string[]>(chatId ? [chatId] : []);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(chatId);

  React.useEffect(() => {
    if (chatId && !openSessions.includes(chatId)) {
      setOpenSessions((prev) => [...prev, chatId]);
      setActiveSessionId(chatId);
    }
  }, [chatId, openSessions]);

  const openSession = (id: string) => {
    setOpenSessions((prev) => (prev.includes(id) ? prev : [...prev, id]));
    setActiveSessionId(id);
  };
  const closeSession = (id: string) => {
    setOpenSessions((prev) => prev.filter((s) => s !== id));
    setActiveSessionId((prev) => (prev === id ? null : prev));
  };

  return (
    <ChatSessionsContext.Provider
      value={{ openSessions, activeSessionId, openSession, closeSession, setActiveSession: setActiveSessionId }}
    >
      {children}
    </ChatSessionsContext.Provider>
  );
}

export default ChatSessionsContainer;