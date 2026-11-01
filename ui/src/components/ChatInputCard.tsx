import React from "react";
import { ChatInput, type ChatAttachment } from "./ChatInput";
import { Card, CardContent } from "./ui/Card";
import { cn } from "../lib/utils";

export function ChatInputCard({
  value,
  onChange,
  onSend,
  onStop,
  disabled,
  sending,
  canStop,
  attachments,
  onAttachmentsChange,
  quotedText,
  onQuoteConsumed,
  searchOpen,
  onSearchToggle,
  onSlashCommand,
  footer,
  className,
}: {
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  onStop?: () => void;
  disabled?: boolean;
  sending?: boolean;
  canStop?: boolean;
  attachments?: ChatAttachment[];
  onAttachmentsChange?: (next: ChatAttachment[]) => void;
  quotedText?: string | null;
  onQuoteConsumed?: () => void;
  searchOpen?: boolean;
  onSearchToggle?: () => void;
  onSlashCommand?: (cmd: { id: string; label: string; description?: string; trigger: string }) => void;
  footer?: React.ReactNode;
  className?: string;
}) {
  return (
    <Card className={cn("border-border-primary", className)}>
      <CardContent className="p-3">
        <ChatInput
          value={value}
          onChange={onChange}
          onSend={onSend}
          onStop={onStop}
          disabled={disabled}
          sending={sending}
          canStop={canStop}
          attachments={attachments}
          onAttachmentsChange={onAttachmentsChange}
          quotedText={quotedText}
          onQuoteConsumed={onQuoteConsumed}
          searchOpen={searchOpen}
          onSearchToggle={onSearchToggle}
          onSlashCommand={onSlashCommand}
        />
        {footer && <div className="mt-2 border-t border-border-primary pt-2">{footer}</div>}
      </CardContent>
    </Card>
  );
}

export default ChatInputCard;