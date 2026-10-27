import React from "react";
import { AutoTextarea } from "./ui/AutoTextarea";
import { Button } from "./ui/Button";
import { cn } from "../lib/utils";
import { defineMessages, useIntl } from "../i18n";

const messages = defineMessages({
  placeholder: { id: "chat.input.placeholder", defaultMessage: "Message Gaggle… (Enter to send)" },
  send: { id: "chat.input.send", defaultMessage: "Send" },
  sending: { id: "chat.input.sending", defaultMessage: "Sending…" },
  stop: { id: "chat.input.stop", defaultMessage: "Stop" },
});

export function ChatInput({
  value,
  onChange,
  onSend,
  onStop,
  disabled,
  sending,
  canStop,
}: {
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  onStop?: () => void;
  disabled?: boolean;
  sending?: boolean;
  canStop?: boolean;
}) {
  const intl = useIntl();
  return (
    <div className="flex flex-col gap-2">
      <AutoTextarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={intl.formatMessage(messages.placeholder)}
        minRows={2}
        maxRows={8}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            if (!disabled && !sending && value.trim()) onSend();
          }
        }}
      />
      <div className="flex items-center justify-end gap-2">
        {canStop && onStop && (
          <Button variant="destructive" size="sm" onClick={onStop}>
            {intl.formatMessage(messages.stop)}
          </Button>
        )}
        <Button variant="default" size="sm" onClick={onSend} disabled={disabled || sending || !value.trim()}>
          {sending ? intl.formatMessage(messages.sending) : intl.formatMessage(messages.send)}
        </Button>
      </div>
    </div>
  );
}
