import React, { useRef, useState, useCallback, useEffect, useMemo } from "react";
import { AutoTextarea } from "./ui/AutoTextarea";
import { Button } from "./ui/Button";
import { Card } from "./ui/Card";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/Tooltip";
import { cn } from "../lib/utils";
import { defineMessages, useIntl } from "../i18n";
import LiveVoiceButton from "./LiveVoiceButton";
import MentionPopover from "./MentionPopover";
import MessageQueue, { type QueuedMessage } from "./MessageQueue";
import { CostTracker } from "./bottom_menu/CostTracker";
import { ContextWindowIndicator } from "./bottom_menu/ContextWindowIndicator";
import { GitBranchIndicator } from "./GitBranchIndicator";
import { DirSwitcher } from "./bottom_menu/DirSwitcher";
import {
  Paperclip,
  Search,
  Slash,
  Quote,
  ArrowUp,
  ArrowDown,
  X,
} from "lucide-react";

const messages = defineMessages({
  placeholder: { id: "chat.input.placeholder", defaultMessage: "Message Gaggle… (Enter to send, / for commands, @ to mention)" },
  send: { id: "chat.input.send", defaultMessage: "Send" },
  sending: { id: "chat.input.sending", defaultMessage: "Sending…" },
  stop: { id: "chat.input.stop", defaultMessage: "Stop" },
  attach: { id: "chat.input.attach", defaultMessage: "Attach file" },
  removeAttachment: { id: "chat.input.removeAttachment", defaultMessage: "Remove attachment" },
  slashCommands: { id: "chat.input.slashCommands", defaultMessage: "Slash commands" },
  search: { id: "chat.input.search", defaultMessage: "Search in conversation" },
  quote: { id: "chat.input.quote", defaultMessage: "Quote selection" },
  queue: { id: "chat.input.queue", defaultMessage: "Queue message" },
  historyPrev: { id: "chat.input.historyPrev", defaultMessage: "Previous command" },
  historyNext: { id: "chat.input.historyNext", defaultMessage: "Next command" },
});

export type ChatAttachment = {
  id: string;
  name: string;
  type: string;
  size: number;
  previewUrl?: string;
  file?: File;
};

export type SlashCommand = { id: string; label: string; description?: string; trigger: string };

const DEFAULT_SLASH_COMMANDS: SlashCommand[] = [
  { id: "compact", label: "/compact", description: "Compact conversation context", trigger: "/compact" },
  { id: "help", label: "/help", description: "Show available commands", trigger: "/help" },
  { id: "clear", label: "/clear", description: "Clear the current draft", trigger: "/clear" },
];

function fileToAttachment(file: File): ChatAttachment {
  return {
    id: `${file.name}-${file.size}-${Date.now()}-${Math.random().toString(36).slice(2)}`,
    name: file.name,
    type: file.type || "application/octet-stream",
    size: file.size,
    file,
  };
}

export function ChatInput({
  value,
  onChange,
  onSend,
  onStop,
  disabled,
  sending,
  canStop,
  attachments = [],
  onAttachmentsChange,
  slashCommands = DEFAULT_SLASH_COMMANDS,
  onSlashCommand,
  mentionItems = [],
  onMentionSelect,
  quotedText,
  onQuoteConsumed,
  searchOpen,
  onSearchToggle,
  showLiveVoice = true,
  commandHistory = [],
  appendQuote,
  onAppendQuoteConsumed,
  totalTokens,
  contextLimit,
  accumulatedInputTokens,
  accumulatedOutputTokens,
  accumulatedCost,
  model,
  provider,
  workingDir,
  onWorkingDirChange,
  droppedFiles = [],
  onFilesProcessed,
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
  slashCommands?: SlashCommand[];
  onSlashCommand?: (cmd: SlashCommand) => void;
  mentionItems?: string[];
  onMentionSelect?: (item: string) => void;
  quotedText?: string | null;
  onQuoteConsumed?: () => void;
  searchOpen?: boolean;
  onSearchToggle?: () => void;
  showLiveVoice?: boolean;
  commandHistory?: string[];
  appendQuote?: string | null;
  onAppendQuoteConsumed?: () => void;
  totalTokens?: number;
  contextLimit?: number;
  accumulatedInputTokens?: number;
  accumulatedOutputTokens?: number;
  accumulatedCost?: number | null;
  model?: string | null;
  provider?: string | null;
  workingDir?: string | null;
  onWorkingDirChange?: (dir: string) => void | Promise<void>;
  droppedFiles?: { name: string; file?: File }[];
  onFilesProcessed?: () => void;
}) {
  const intl = useIntl();
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const textAreaRef = useRef<HTMLTextAreaElement | null>(null);
  const [slashOpen, setSlashOpen] = useState(false);
  const [slashFilter, setSlashFilter] = useState("");
  const [slashIndex, setSlashIndex] = useState(0);
  const [mentionOpen, setMentionOpen] = useState(false);
  const [mentionQuery, setMentionQuery] = useState("");
  const [mentionStart, setMentionStart] = useState(-1);
  const [mentionIndex, setMentionIndex] = useState(0);
  const [queuedMessages, setQueuedMessages] = useState<QueuedMessage[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState("");

  // Derived
  const filteredSlash = slashCommands.filter((c) =>
    c.label.toLowerCase().startsWith(slashFilter || "/"),
  );
  const filteredMentions = mentionItems.filter((i) =>
    i.toLowerCase().includes(mentionQuery.toLowerCase()),
  );

  // Append quote handling (Goose parity)
  useEffect(() => {
    if (!appendQuote) return;
    const blockquote = appendQuote
      .split("\n")
      .map((line) => `> ${line}`)
      .join("\n");
    const next = value ? `${value}\n\n${blockquote}\n\n` : `${blockquote}\n\n`;
    onChange(next);
    setTimeout(() => textAreaRef.current?.focus(), 0);
    onAppendQuoteConsumed?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [appendQuote]);

  // quotedText (BaseChat parity)
  useEffect(() => {
    if (!quotedText) return;
    const next = value ? `${value}\n> ${quotedText}\n` : `> ${quotedText}\n`;
    onChange(next);
    onQuoteConsumed?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [quotedText]);

  // Slash detection
  useEffect(() => {
    const trimmed = value.trimStart();
    if (trimmed.startsWith("/")) {
      const token = trimmed.split(/\s/)[0];
      setSlashFilter(token.toLowerCase());
      setSlashOpen(true);
      setSlashIndex(0);
    } else {
      setSlashOpen(false);
      setSlashFilter("");
    }
  }, [value]);

  // Mention detection (@)
  useEffect(() => {
    const idx = value.lastIndexOf("@");
    if (idx >= 0) {
      const after = value.slice(idx + 1);
      if (!after.includes(" ") && !after.includes("\n")) {
        setMentionOpen(true);
        setMentionQuery(after);
        setMentionStart(idx);
        setMentionIndex(0);
        return;
      }
    }
    setMentionOpen(false);
    setMentionQuery("");
    setMentionStart(-1);
  }, [value]);

  // Process dropped files
  useEffect(() => {
    if (droppedFiles && droppedFiles.length > 0 && onAttachmentsChange) {
      const next = [...attachments];
      droppedFiles.forEach((df) => {
        if (df.file) next.push(fileToAttachment(df.file));
      });
      onAttachmentsChange(next);
      onFilesProcessed?.();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [droppedFiles]);

  // Image paste handling
  const handlePaste = useCallback(
    (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
      const items = e.clipboardData?.items;
      if (!items || !onAttachmentsChange) return;
      const files: File[] = [];
      for (let i = 0; i < items.length; i++) {
        const it = items[i];
        if (it.kind === "file") {
          const f = it.getAsFile();
          if (f) files.push(f);
        }
      }
      if (files.length === 0) return;
      e.preventDefault();
      const next = [...attachments];
      files.forEach((f) => {
        const att = fileToAttachment(f);
        if (f.type.startsWith("image/")) {
          const reader = new FileReader();
          reader.onload = () => {
            att.previewUrl = reader.result as string;
            onAttachmentsChange([...next]);
          };
          reader.readAsDataURL(f);
        }
        next.push(att);
      });
      onAttachmentsChange(next);
    },
    [attachments, onAttachmentsChange],
  );

  const handleFiles = (files: FileList | null) => {
    if (!files || !onAttachmentsChange) return;
    const next: ChatAttachment[] = [...attachments];
    Array.from(files).forEach((file) => {
      if (next.length >= 8) return;
      const att = fileToAttachment(file);
      if (file.type.startsWith("image/")) {
        const reader = new FileReader();
        reader.onload = () => {
          att.previewUrl = reader.result as string;
          onAttachmentsChange([...next]);
        };
        reader.readAsDataURL(file);
      }
      next.push(att);
    });
    onAttachmentsChange(next);
  };

  const removeAttachment = (id: string) => {
    onAttachmentsChange?.(attachments.filter((a) => a.id !== id));
  };

  const handleSlashSelect = (cmd: SlashCommand) => {
    setSlashOpen(false);
    onSlashCommand?.(cmd);
    if (cmd.trigger === "/clear") onChange("");
    const rest = value.trimStart().slice(slashFilter.length);
    onChange(cmd.trigger + " " + rest.replace(/^\s*/, ""));
  };

  const handleMentionSelect = (item: string) => {
    setMentionOpen(false);
    onMentionSelect?.(item);
    if (mentionStart >= 0) {
      const before = value.slice(0, mentionStart);
      const after = value.slice(mentionStart + 1 + mentionQuery.length);
      onChange(`${before}@${item}${after}`);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Popover navigation
    if (slashOpen && filteredSlash.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSlashIndex((i) => (i + 1) % filteredSlash.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSlashIndex((i) => (i - 1 + filteredSlash.length) % filteredSlash.length);
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        handleSlashSelect(filteredSlash[slashIndex]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        setSlashOpen(false);
        return;
      }
    }
    if (mentionOpen && filteredMentions.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setMentionIndex((i) => (i + 1) % filteredMentions.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setMentionIndex((i) => (i - 1 + filteredMentions.length) % filteredMentions.length);
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        handleMentionSelect(filteredMentions[mentionIndex]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        setMentionOpen(false);
        return;
      }
    }
    // Command history navigation
    if (e.key === "ArrowUp" && !value.trim() && commandHistory.length > 0) {
      e.preventDefault();
      if (historyIndex < commandHistory.length - 1) {
        const newIndex = historyIndex + 1;
        setHistoryIndex(newIndex);
        if (historyIndex === -1) setSavedInput(value);
        onChange(commandHistory[commandHistory.length - 1 - newIndex]);
      }
      return;
    }
    if (e.key === "ArrowDown" && historyIndex >= 0) {
      e.preventDefault();
      const newIndex = historyIndex - 1;
      setHistoryIndex(newIndex);
      if (newIndex === -1) {
        onChange(savedInput);
      } else {
        onChange(commandHistory[commandHistory.length - 1 - newIndex]);
      }
      return;
    }

    if (e.key === "Enter" && !e.shiftKey && !slashOpen && !mentionOpen) {
      e.preventDefault();
      if (!disabled && !sending && (value.trim() || attachments.length)) {
        // Queue if busy
        if (sending) {
          const qm: QueuedMessage = {
            id: `${Date.now()}`,
            content: value,
            images: attachments
              .filter((a) => a.previewUrl)
              .map((a) => ({ dataUrl: a.previewUrl!, mimeType: a.type })),
          };
          setQueuedMessages((prev) => [...prev, qm]);
          onChange("");
          onAttachmentsChange?.([]);
        } else {
          onSend();
        }
      }
    }
    if (e.key === "Escape") {
      if (slashOpen) { e.preventDefault(); setSlashOpen(false); }
      if (mentionOpen) { e.preventDefault(); setMentionOpen(false); }
    }
  };

  const handleQueueClear = () => setQueuedMessages([]);
  const handleQueueSteer = (id: string) => {
    const msg = queuedMessages.find((m) => m.id === id);
    if (msg) {
      onChange(msg.content);
      setQueuedMessages((prev) => prev.filter((m) => m.id !== id));
    }
  };

  const canSend = !disabled && !sending && (value.trim().length > 0 || attachments.length > 0);

  return (
    <div className="flex flex-col gap-2 relative">
      {/* Attachments */}
      {attachments.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {attachments.map((a) => (
            <div key={a.id} className="relative group rounded-md border border-border-primary bg-background-secondary p-1">
              {a.previewUrl ? (
                <img src={a.previewUrl} alt={a.name} className="h-16 w-16 object-cover rounded" />
              ) : (
                <div className="h-16 w-24 flex items-center justify-center text-xs text-text-secondary px-2 text-center break-all">
                  {a.name}
                </div>
              )}
              <button
                type="button"
                onClick={() => removeAttachment(a.id)}
                className="absolute -top-1 -right-1 h-5 w-5 rounded-full bg-background-primary border border-border-primary text-xs text-text-primary opacity-0 group-hover:opacity-100"
                title={intl.formatMessage(messages.removeAttachment)}
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Slash commands popover */}
      {slashOpen && filteredSlash.length > 0 && (
        <Card className="absolute bottom-full mb-2 z-50 w-72 shadow-lg">
          <div className="p-1">
            {filteredSlash.map((c, i) => (
              <button
                key={c.id}
                type="button"
                onClick={() => handleSlashSelect(c)}
                className={`w-full text-left px-2 py-1.5 rounded ${i === slashIndex ? "bg-background-secondary" : "hover:bg-background-secondary"}`}
              >
                <div className="text-sm font-medium text-text-primary">{c.label}</div>
                {c.description && <div className="text-xs text-text-secondary">{c.description}</div>}
              </button>
            ))}
          </div>
        </Card>
      )}

      {/* Mention popover */}
      {mentionOpen && filteredMentions.length > 0 && (
        <Card className="absolute bottom-full mb-2 z-50 w-64 shadow-lg">
          <div className="p-1 max-h-48 overflow-y-auto">
            {filteredMentions.map((m, i) => (
              <button
                key={m}
                type="button"
                onClick={() => handleMentionSelect(m)}
                className={`w-full text-left px-2 py-1.5 rounded text-sm text-text-primary ${i === mentionIndex ? "bg-background-secondary" : "hover:bg-background-secondary"}`}
              >
                {m}
              </button>
            ))}
          </div>
        </Card>
      )}

      {/* Queued messages */}
      {queuedMessages.length > 0 && (
        <MessageQueue
          count={queuedMessages.length}
          items={queuedMessages}
          onClear={handleQueueClear}
          onSteer={handleQueueSteer}
        />
      )}

      <AutoTextarea
        ref={textAreaRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={intl.formatMessage(messages.placeholder)}
        minRows={2}
        maxRows={8}
        onKeyDown={handleKeyDown}
        onPaste={handlePaste}
      />

      {/* Bottom bar */}
      <div className="flex items-center justify-between gap-2 flex-wrap">
        <div className="flex items-center gap-2 flex-wrap">
          <input
            ref={fileInputRef}
            type="file"
            multiple
            className="hidden"
            onChange={(e) => {
              handleFiles(e.target.files);
              if (fileInputRef.current) fileInputRef.current.value = "";
            }}
          />
          <Button variant="ghost" size="sm" type="button" disabled={disabled || sending} title={intl.formatMessage(messages.attach)} onClick={() => fileInputRef.current?.click()}>
            <Paperclip className="w-4 h-4" />
          </Button>
          <Button variant="ghost" size="sm" type="button" title={intl.formatMessage(messages.slashCommands)} onClick={() => { if (!value.trim().startsWith("/")) onChange("/" + value); setSlashOpen(true); }}>
            <Slash className="w-4 h-4" />
          </Button>
          <Button variant="ghost" size="sm" type="button" title={intl.formatMessage(messages.quote)} onClick={() => { const sel = window.getSelection?.()?.toString(); if (sel) onChange(value ? `${value}\n> ${sel}\n` : `> ${sel}\n`); }}>
            <Quote className="w-4 h-4" />
          </Button>
          {onSearchToggle && (
            <Button variant={searchOpen ? "default" : "ghost"} size="sm" type="button" title={intl.formatMessage(messages.search)} onClick={onSearchToggle}>
              <Search className="w-4 h-4" />
            </Button>
          )}
          <DirSwitcher currentDir={workingDir} onSwitch={onWorkingDirChange} />
          <GitBranchIndicator branch={null} />
          {showLiveVoice && <LiveVoiceButton />}
          {model && provider && (
            <CostTracker
              inputTokens={accumulatedInputTokens || 0}
              outputTokens={accumulatedOutputTokens || 0}
              accumulatedCost={accumulatedCost}
              model={model}
              provider={provider}
            />
          )}
          {totalTokens != null && contextLimit != null && (
            <ContextWindowIndicator totalTokens={totalTokens} contextLimit={contextLimit} />
          )}
        </div>
        <div className="flex items-center gap-2">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="sm" type="button" onClick={() => { if (historyIndex < commandHistory.length - 1) { const ni = historyIndex + 1; setHistoryIndex(ni); if (historyIndex === -1) setSavedInput(value); onChange(commandHistory[commandHistory.length - 1 - ni]); } }} title={intl.formatMessage(messages.historyPrev)}>
                <ArrowUp className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{intl.formatMessage(messages.historyPrev)}</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="sm" type="button" onClick={() => { if (historyIndex >= 0) { const ni = historyIndex - 1; setHistoryIndex(ni); if (ni === -1) onChange(savedInput); else onChange(commandHistory[commandHistory.length - 1 - ni]); } }} title={intl.formatMessage(messages.historyNext)}>
                <ArrowDown className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{intl.formatMessage(messages.historyNext)}</TooltipContent>
          </Tooltip>
          {canStop && onStop && (
            <Button variant="destructive" size="sm" onClick={onStop}>
              {intl.formatMessage(messages.stop)}
            </Button>
          )}
          <Button variant="default" size="sm" onClick={onSend} disabled={!canSend}>
            {sending ? intl.formatMessage(messages.sending) : intl.formatMessage(messages.send)}
          </Button>
        </div>
      </div>
    </div>
  );
}