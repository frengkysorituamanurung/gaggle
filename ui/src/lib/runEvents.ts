import type { RunEvent, RunEventKind, RunEventPayload, TypedRunEvent } from "../types";

export const RUN_EVENT_KINDS: readonly RunEventKind[] = [
  "run.started",
  "run.completed",
  "run.cancelled",
  "run.failed",
  "run.persist",
  "message.id",
  "message.role",
  "message.delta",
  "tool.started",
  "tool.completed",
  "tool.failed",
  "tool.denied",
  "approval.requested",
  "approval.approved",
];

export function parseRunEvent(
  kind: string,
  data: string,
  id?: string | null,
): TypedRunEvent | null {
  const trimmed = data.trim();
  if (trimmed === "") return null;
  let payload: RunEventPayload;
  try {
    const parsed = JSON.parse(trimmed);
    payload = parsed && typeof parsed === "object" ? (parsed as RunEventPayload) : { value: parsed };
  } catch {
    return null;
  }
  const seq = typeof payload.seq === "number" ? payload.seq : Number(id ?? 0) || 0;
  return {
    seq,
    id: id ?? undefined,
    kind,
    payload,
    raw: trimmed,
  };
}

export function fromRunEvent(record: RunEvent): TypedRunEvent | null {
  return parseRunEvent(record.kind, record.payload, record.id != null ? String(record.id) : null);
}

export type RunEventHandler = (event: TypedRunEvent) => void;

export type RunEventHandlers = Partial<Record<RunEventKind | string, RunEventHandler>> & {
  onAny?: RunEventHandler;
  onError?: (error: Event) => void;
  onOpen?: () => void;
};

export function subscribeToRunEvents(
  runId: string,
  handlers: RunEventHandlers = {},
  afterSeq = 0,
): () => void {
  const url = `/api/runs/${encodeURIComponent(runId)}/events?after_seq=${afterSeq}`;
  const source = new EventSource(url);

  const dispatch = (kind: string, ev: MessageEvent<string>) => {
    const parsed = parseRunEvent(kind, ev.data, ev.lastEventId || null);
    if (!parsed) return;
    handlers.onAny?.(parsed);
    const specific = handlers[kind as keyof RunEventHandlers];
    if (typeof specific === "function") {
      (specific as RunEventHandler)(parsed);
    }
    const wildcard = handlers["*"];
    if (typeof wildcard === "function") {
      (wildcard as RunEventHandler)(parsed);
    }
  };

  for (const kind of RUN_EVENT_KINDS) {
    source.addEventListener(kind, (ev) => dispatch(kind, ev as MessageEvent<string>));
  }
  source.onmessage = (ev) => dispatch("message", ev);

  if (handlers.onOpen) source.onopen = () => handlers.onOpen?.();
  if (handlers.onError) source.onerror = (ev) => handlers.onError?.(ev);

  return () => {
    source.close();
  };
}

export type { RunEventKind, RunEventPayload, TypedRunEvent } from "../types";
