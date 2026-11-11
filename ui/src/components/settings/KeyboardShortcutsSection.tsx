import React, { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card";
import { Button } from "../ui/Button";

interface Shortcut {
  action: string;
  keys: string;
  description: string;
}

const SHORTCUTS: Shortcut[] = [
  { action: "newChat", keys: "⌘N", description: "New chat" },
  { action: "quickLauncher", keys: "⌘K", description: "Quick launcher" },
  { action: "settings", keys: "⌘,", description: "Open settings" },
  { action: "history", keys: "⌘H", description: "Chat history" },
  { action: "cancel", keys: "Escape", description: "Cancel / close" },
  { action: "submit", keys: "Enter", description: "Submit message" },
  { action: "newline", keys: "Shift+Enter", description: "New line in composer" },
];

export default function KeyboardShortcutsSection() {
  const [editing, setEditing] = useState<string | null>(null);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-text-primary text-base">Keyboard Shortcuts</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="text-sm text-text-secondary mb-2">
          Customize keyboard shortcuts. Click a shortcut to rebind.
        </div>
        <table className="w-full text-sm">
          <thead>
            <tr className="text-text-tertiary text-xs uppercase">
              <th className="text-left py-1">Action</th>
              <th className="text-left py-1">Shortcut</th>
              <th className="text-left py-1">Description</th>
            </tr>
          </thead>
          <tbody>
            {SHORTCUTS.map((s) => (
              <tr key={s.action} className="border-t border-border-primary">
                <td className="py-2 text-text-primary">{s.action}</td>
                <td className="py-2">
                  <kbd className="px-2 py-1 text-xs rounded bg-surface-2 text-text-secondary font-mono">
                    {s.keys}
                  </kbd>
                </td>
                <td className="py-2 text-text-secondary">{s.description}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </CardContent>
    </Card>
  );
}