import React from "react";
import type * as T from "../types";
import { MessageBubble } from "./MessageBubble";
import Markdown from "./Markdown";
import { ToolCallCard } from "./ToolCallCard";
import { ThinkingBlock } from "./ThinkingBlock";

export function MessageList({
  messages = [],
  partsByMessage = new Map(),
}: {
  messages?: T.Message[];
  partsByMessage?: Map<string, T.MessagePart[]>;
}) {
  return (
    <div className="space-y-4">
      {messages.map((m) => {
        const parts = (partsByMessage.get(m.id) || []).sort((a, b) => a.idx - b.idx);
        return (
          <MessageBubble key={m.id} role={m.role as "user" | "assistant" | "system" | "tool"}>
            {parts.map((p) => {
              if (p.kind === "text") return <Markdown key={p.id} text={p.content || ""} />;
              if (p.kind === "reasoning") return <ThinkingBlock key={p.id} content={p.content} />;
              if (p.kind === "tool_call") return <ToolCallCard key={p.id} label="Tool call" content={p.content} />;
              if (p.kind === "tool_result") return <ToolCallCard key={p.id} label="Tool result" content={p.content} />;
              return null;
            })}
            {parts.length === 0 && m.role === "assistant" && <span className="text-text-secondary text-sm">(empty)</span>}
          </MessageBubble>
        );
      })}
    </div>
  );
}

export default MessageList;