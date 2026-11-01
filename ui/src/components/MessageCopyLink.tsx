import React from "react";
import { Button } from "./ui/Button";
import { Copy } from "lucide-react";

export function MessageCopyLink({ text }: { text: string }) {
  return (
    <Button
      size="sm"
      variant="ghost"
      onClick={() => navigator.clipboard?.writeText(text)}
      title="Copy"
    >
      <Copy className="w-3 h-3" />
    </Button>
  );
}

export default MessageCopyLink;