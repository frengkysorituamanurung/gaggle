import React from "react";
import { cn } from "../lib/utils";
import Markdown from "./Markdown";

export function MessageBubble({
  role,
  children,
}: {
  role: "user" | "assistant" | "system" | "tool";
  children: React.ReactNode;
}) {
  const isUser = role === "user";
  return (
    <div className={`flex ${isUser ? "justify-end" : "justify-start"}`}>
      <div
        className={cn(
          "max-w-[85%] rounded-lg px-3 py-2",
          isUser ? "bg-background-tertiary text-text-primary" : "bg-background-secondary text-text-primary"
        )}
      >
        {children}
      </div>
    </div>
  );
}

export default MessageBubble;
