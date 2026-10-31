import React, { useState, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { Search, X } from "lucide-react";

interface QuickLauncherProps {
  open: boolean;
  onClose: () => void;
}

const ACTIONS = [
  { label: "New Chat", path: "/chat", shortcut: "⌘N" },
  { label: "Settings", path: "/settings", shortcut: "⌘," },
  { label: "History", path: "/history", shortcut: "⌘H" },
  { label: "Recipes", path: "/recipes", shortcut: "" },
  { label: "Extensions", path: "/extensions", shortcut: "" },
  { label: "Memories", path: "/memories", shortcut: "" },
  { label: "Scheduler", path: "/scheduler", shortcut: "" },
  { label: "Diagnostics", path: "/diagnostics", shortcut: "" },
  { label: "Worktrees", path: "/worktrees", shortcut: "" },
  { label: "Terminal", path: "/terminal", shortcut: "" },
  { label: "Workspace", path: "/workspace", shortcut: "" },
];

export default function QuickLauncher({ open, onClose }: QuickLauncherProps) {
  const [query, setQuery] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const navigate = useNavigate();

  useEffect(() => {
    if (open) {
      setQuery("");
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape" && open) onClose();
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        if (!open) onClose(); // toggle handled by parent
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onClose]);

  if (!open) return null;

  const filtered = ACTIONS.filter((a) =>
    a.label.toLowerCase().includes(query.toLowerCase())
  );

  const select = (path: string) => {
    navigate(path);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh] bg-black/50">
      <div className="w-full max-w-md bg-background-primary rounded-lg shadow-xl border border-border-primary overflow-hidden">
        <div className="flex items-center gap-2 px-4 py-3 border-b border-border-primary">
          <Search size={16} className="text-text-tertiary" />
          <input
            ref={inputRef}
            className="flex-1 bg-transparent text-text-primary text-sm outline-none placeholder:text-text-tertiary"
            placeholder="Search actions..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <button onClick={onClose} className="text-text-tertiary hover:text-text-primary">
            <X size={16} />
          </button>
        </div>
        <div className="max-h-64 overflow-y-auto">
          {filtered.map((a) => (
            <button
              key={a.path}
              className="w-full flex items-center justify-between px-4 py-2 text-sm text-text-primary hover:bg-surface-1 transition-colors"
              onClick={() => select(a.path)}
            >
              <span>{a.label}</span>
              {a.shortcut && <span className="text-xs text-text-tertiary">{a.shortcut}</span>}
            </button>
          ))}
          {filtered.length === 0 && (
            <div className="px-4 py-3 text-sm text-text-secondary">No matching actions.</div>
          )}
        </div>
      </div>
    </div>
  );
}