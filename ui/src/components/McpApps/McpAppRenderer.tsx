import React, { useEffect, useRef, useState } from "react";
import { Card, CardContent } from "../ui/Card";

interface McpAppRendererProps {
  url: string;
  title?: string;
  onClose?: () => void;
}

export default function McpAppRenderer({ url, title, onClose }: McpAppRendererProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    const timer = setTimeout(() => setLoading(false), 3000);
    return () => clearTimeout(timer);
  }, [url]);

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
          sandbox="allow-scripts allow-same-origin allow-forms"
          onLoad={() => setLoading(false)}
          onError={() => setError("Failed to load MCP app")}
          title={title || "MCP App"}
        />
      </CardContent>
    </Card>
  );
}