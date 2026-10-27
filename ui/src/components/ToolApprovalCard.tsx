import React, { useState } from "react";
import { Button } from "./ui/Button";
import { cn } from "../lib/utils";
import { defineMessages, useIntl } from "../i18n";

const messages = defineMessages({
  title: { id: "approval.title", defaultMessage: "Approval required" },
  approve: { id: "approval.approve", defaultMessage: "Approve" },
  deny: { id: "approval.deny", defaultMessage: "Deny" },
  remember: { id: "approval.remember", defaultMessage: "Remember this command" },
  exact: { id: "approval.match.exact", defaultMessage: "exact" },
  prefix: { id: "approval.match.prefix", defaultMessage: "prefix" },
});

export function ToolApprovalCard({
  approvalId,
  toolName,
  summary,
  command,
  cwd,
  onDecide,
}: {
  approvalId: string;
  toolName?: string;
  summary?: string;
  command?: string | null;
  cwd?: string | null;
  onDecide: (approved: boolean, remember?: { pattern: string; matchKind: string }) => void;
}) {
  const intl = useIntl();
  const [remember, setRemember] = useState(false);
  const [matchKind, setMatchKind] = useState<"exact" | "prefix">("exact");

  return (
    <div className="mt-3 rounded-md border border-border-primary bg-background-secondary p-3 text-sm">
      <div className="font-semibold text-text-primary mb-1">{intl.formatMessage(messages.title)}</div>
      {toolName && <div className="text-text-secondary text-xs mb-1">{toolName}</div>}
      {summary && <div className="text-text-primary mb-2">{summary}</div>}
      {command && (
        <pre className="mb-2 px-2 py-1 rounded bg-background-primary border border-border-primary text-xs overflow-x-auto">
          {command}
        </pre>
      )}
      {cwd && <div className="text-xs text-text-tertiary mb-2">cwd: {cwd}</div>}
      <label className="flex items-center gap-2 text-xs text-text-secondary mb-2">
        <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
        {intl.formatMessage(messages.remember)}
        {remember && (
          <select
            value={matchKind}
            onChange={(e) => setMatchKind(e.target.value as "exact" | "prefix")}
            className="ml-2 px-1 py-0.5 rounded border border-border-primary bg-background-primary text-text-primary"
          >
            <option value="exact">{intl.formatMessage(messages.exact)}</option>
            <option value="prefix">{intl.formatMessage(messages.prefix)}</option>
          </select>
        )}
      </label>
      <div className="flex gap-2">
        <Button size="sm" variant="default" onClick={() => onDecide(true, remember && command ? { pattern: command, matchKind } : undefined)}>
          {intl.formatMessage(messages.approve)}
        </Button>
        <Button size="sm" variant="destructive" onClick={() => onDecide(false)}>
          {intl.formatMessage(messages.deny)}
        </Button>
      </div>
    </div>
  );
}
