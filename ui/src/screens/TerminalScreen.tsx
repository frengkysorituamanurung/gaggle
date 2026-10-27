import React, { useEffect, useRef, useState } from "react";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";

export default function TerminalScreen() {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [output, setOutput] = useState("");
  const [input, setInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);

  const create = async () => {
    try {
      const res = await fetch("/api/terminal-sessions", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({}) });
      const data = await res.json();
      const id = data.id || data.session_id;
      setSessionId(id);
      // connect websocket
      const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
      const ws = new WebSocket(`${proto}//${window.location.host}/api/terminal-sessions/${id}/ws`);
      ws.onmessage = (ev) => setOutput((prev) => prev + ev.data);
      ws.onerror = () => setError("WebSocket error");
      wsRef.current = ws;
    } catch (e) { setError(String(e)); }
  };

  const send = async () => {
    if (!sessionId || !input) return;
    try {
      await fetch(`/api/terminal-sessions/${sessionId}/input`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ input: input + "\n" }) });
      setInput("");
    } catch (e) { setError(String(e)); }
  };

  const close = async () => {
    if (sessionId) await fetch(`/api/terminal-sessions/${sessionId}`, { method: "DELETE" });
    wsRef.current?.close();
    setSessionId(null); setOutput("");
  };

  useEffect(() => () => { wsRef.current?.close(); }, []);

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Terminal</h2>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
      {!sessionId ? (
        <Button variant="default" onClick={create}>New Terminal Session</Button>
      ) : (
        <Card>
          <CardHeader><CardTitle className="text-text-primary text-base">Session {sessionId}</CardTitle></CardHeader>
          <CardContent>
            <pre className="text-xs bg-code-bg p-3 rounded-md overflow-auto max-h-96 whitespace-pre-wrap">{output}</pre>
            <div className="flex gap-2 mt-3">
              <Input value={input} onChange={(e) => setInput(e.target.value)} onKeyDown={(e) => { if (e.key === "Enter") send(); }} placeholder="command" />
              <Button variant="default" size="sm" onClick={send}>Send</Button>
              <Button variant="destructive" size="sm" onClick={close}>Close</Button>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}