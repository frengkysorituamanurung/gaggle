import React from "react";
import Markdown from "./Markdown";

export function GooseMessage({ content }: { content: string }) {
  return (
    <div className="flex justify-start">
      <div className="max-w-[85%] rounded-lg px-3 py-2 bg-background-secondary text-text-primary">
        <Markdown text={content} />
      </div>
    </div>
  );
}

export default GooseMessage;