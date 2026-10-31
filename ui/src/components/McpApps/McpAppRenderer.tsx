import React, { useEffect, useRef, useState, useCallback } from "react";
import { Card, CardContent } from "../ui/Card";

interface McpAppRendererProps {
  url: string;
  title?: string;
  onClose?: () => void;
  onMessage?: (msg: unknown) => void;
}

interface McpAppMessage {
  type: "tool_result" | "notification" | "error";
  payload: unknown;
}

export default function McpAppRenderer({ url, title, onClose, onMessage }: McpAppRendererProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [messages, setMessages] = useState<McpAppMessage[]>([]);

  useEffect(() => {
    setLoading(true);
    setError(null);
    const timer = setTimeout(() => setLoading(false), 3000);
    return () => clearTimeout(timer);
  }, [url]);

  // Listen for messages from the iframe
  useEffect(() => {
    const handler = (event: MessageEvent) => {
      // Validate origin
      try {
        const origin = new URL(url).origin;
        if (event.origin !== origin && event.origin !== "null") {
          return;
        }
      } catch {
        return;
      }

      if (event.data && typeof event.data === "object") {
        const msg = event.data as McpAppMessage;
        setMessages((prev) => [...prev, msg]);
        onMessage?.(msg);
      }
    };
    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, [url, onMessage]);

  const sendMessage = useCallback((msg: unknown) => {
    if (iframeRef.current?.contentWindow) {
      iframeRef.current.contentWindow.postMessage(msg, "*");
    }
  }, []);

  if (!url) {
    return (
      <Card>
        <CardContent className="p-4 text-text-secondary text-sm">
          No MCP app URL provided.
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="overflow-hidden">
      <div className="flex items-center justify-between px-4 py-2 border-b border-border-primary">
        <span className="text-sm font-medium text-text-primary">{title || "MCP App"}</span>
        {onClose && (
          <button onClick={onClose} className="text-text-tertiary hover:text-text-primary text-sm">
            Close
          </button>
        )}
      </div>
      <CardContent className="p-0">
        {loading && (
          <div className="flex items-center justify-center h-48 text-text-secondary text-sm">
            Loading MCP app...
          </div>
        )}
        {error && (
          <div className="flex items-center justify-center h-48 text-text-danger text-sm">
            {error}
          </div>
        )}
        <iframe
          ref={iframeRef}
          src={url}
          className="w-full h-96 border-0"
          sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
          onLoad={() => setLoading(false)}
          onError={() => setError("Failed to load MCP app")}
          title={title || "MCP App"}
        />
        {messages.length > 0 && (
          <div className="border-t border-border-primary p-2 max-h-32 overflow-y-auto">
            <div className="text-xs text-text-tertiary mb-1">Messages ({messages.length})</div>
            {messages.slice(-5).map((m, i) => (
              <div key={i} className="text-xs text-text-secondary truncate">
                {m.type}: {JSON.stringify(m.payload).slice(0, 100)}
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}