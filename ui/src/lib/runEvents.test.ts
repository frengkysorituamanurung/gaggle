import { describe, expect, it } from "vitest";
import { parseRunEvent, fromRunEvent, RUN_EVENT_KINDS } from "./runEvents";
import type { RunEvent } from "../types";

describe("runEvents adapter", () => {
  it("parses a valid JSON payload", () => {
    const ev = parseRunEvent("message.delta", JSON.stringify({ text: "hi", seq: 3 }), "7");
    expect(ev).not.toBeNull();
    expect(ev?.kind).toBe("message.delta");
    expect(ev?.seq).toBe(3);
    expect(ev?.payload.text).toBe("hi");
    expect(ev?.raw).toContain("hi");
  });

  it("falls back to event id for seq", () => {
    const ev = parseRunEvent("run.started", JSON.stringify({}), "42");
    expect(ev?.seq).toBe(42);
  });

  it("returns null for empty or invalid data", () => {
    expect(parseRunEvent("message.delta", "   ")).toBeNull();
    expect(parseRunEvent("message.delta", "{not json}")).toBeNull();
  });

  it("wraps non-object payloads", () => {
    const ev = parseRunEvent("run.started", "42");
    expect(ev?.payload).toEqual({ value: 42 });
  });

  it("parses backend RunEvent records", () => {
    const record: RunEvent = {
      id: 9,
      seq: 9,
      kind: "tool.started",
      payload: JSON.stringify({ tool_name: "shell" }),
    };
    const ev = fromRunEvent(record);
    expect(ev?.kind).toBe("tool.started");
    expect(ev?.payload.tool_name).toBe("shell");
  });

  it("exposes all canonical kinds", () => {
    expect(RUN_EVENT_KINDS).toContain("message.delta");
    expect(RUN_EVENT_KINDS).toContain("approval.requested");
  });
});
