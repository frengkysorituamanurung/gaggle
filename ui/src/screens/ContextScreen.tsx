import React, { useEffect, useState } from "react";
import { api } from "../lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";
import { Button } from "../components/ui/Button";

interface ContextScreenProps {
  chatId: string;
}

export default function ContextScreen({ chatId }: ContextScreenProps) {
  const [messages, setMessages] = useState<unknown[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    if (!chatId) return;
    setLoading(true);
    try {
      const msgs = await api.chats.messages(chatId);
      setMessages(msgs);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, [chatId]);

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Context Management</h2>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
      <div className="text-sm text-text-secondary mb-4">
        {messages.length} messages in context
      </div>
      <Button size="sm" onClick={load} disabled={loading} className="mb-4">
        {loading ? "Loading..." : "Refresh"}
      </Button>
      <Card>
        <CardHeader>
          <CardTitle className="text-text-primary text-base">Context Budget</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-text-secondary text-sm">
            Context compaction and token budget management. Use the chat's compact
            feature to summarize older messages.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}