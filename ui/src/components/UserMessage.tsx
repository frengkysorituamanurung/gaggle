import React from "react";
import Markdown from "./Markdown";

export function UserMessage({ content }: { content: string }) {
  return (
    <div className="flex justify-end">
      <div className="max-w-[85%] rounded-lg px-3 py-2 bg-background-tertiary text-text-primary">
        <Markdown text={content} />
      </div>
    </div>
  );
}

export default UserMessage;><|start|><|start|>write to=assistant<|channel|>analysis<|message|>Created file successfully: ui/src/components/UserMessage.tsx