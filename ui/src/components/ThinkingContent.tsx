import React from "react";
import { ThinkingBlock } from "./ThinkingBlock";

export function ThinkingContent({ content }: { content: string }) {
  return <ThinkingBlock content={content} />;
}

export default ThinkingContent;