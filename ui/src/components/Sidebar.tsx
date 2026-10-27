import { useEffect, useState } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";

const NAV_ITEMS: { path: string; label: string; icon: string }[] = [
  { path: "/chat", label: "New Chat", icon: "💬" },
  { path: "/recipes", label: "Recipes", icon: "📜" },
  { path: "/skills", label: "Skills", icon: "🧩" },
  { path: "/extensions", label: "Extensions", icon: "🔌" },
  { path: "/scheduler", label: "Scheduler", icon: "⏰" },
  { path: "/history", label: "Session History", icon: "🗂" },
];

export default function Sidebar() {
  const navigate = useNavigate();
  const location = useLocation();
  const { theme, setTheme, projectId, setChatId } = useApp();
  const [collapsed, setCollapsed] = useState(false);
  const [chats, setChats] = useState<T.Chat[]>([]);
  const [isChatsExpanded, setIsChatsExpanded] = useState(true);

  useEffect(() => {
    if (!projectId) return;
    api.projectChats(projectId).then(setChats).catch(() => {});
  }, [projectId]);

  return (
    <div className="relative flex-shrink-0 overflow-hidden h-full p-2">
      <div className="w-full h-full overflow-hidden rounded-xl border border-border-primary bg-background-primary">
        <div className="flex flex-col h-full">
          <div className="h-[48px] no-drag" />
          <div className="px-2 flex flex-col gap-0.5">
            {NAV_ITEMS.map((n) => {
              const active = location.pathname === n.path;
              return (
                <button
                  key={n.path}
                  onClick={() => navigate(n.path)}
                  title={n.label}
                  className={`flex flex-row items-center gap-3 outline-none no-drag w-full rounded-full px-3 py-2 text-sm font-medium transition-colors ${
                    active
                      ? "bg-background-tertiary text-text-primary"
                      : "text-text-primary hover:bg-background-tertiary/60"
                  }`}
                >
                  <span className="w-5 h-5 flex-shrink-0 text-text-secondary text-center">{n.icon}</span>
                  {!collapsed && <span className="text-left flex-1 truncate">{n.label}</span>}
                </button>
              );
            })}
          </div>

          {!collapsed && (
            <div className="flex-1 min-h-0 flex flex-col mt-3">
              <button
                onClick={() => setIsChatsExpanded((v) => !v)}
                className="px-3 pt-2 pb-0.5 text-[10px] uppercase tracking-wider text-text-tertiary flex items-center gap-1"
              >
                {isChatsExpanded ? "▾" : "▸"} Chats
              </button>
              {isChatsExpanded && (
                <div className="flex-1 min-h-0 overflow-y-auto px-2 pb-2 mt-1">
                  {chats.length === 0 && (
                    <div className="text-xs text-text-secondary px-3 py-1.5">No recent chats</div>
                  )}
                  {chats.map((c) => (
                    <button
                      key={c.id}
                      onClick={() => {
                        setChatId(c.id);
                        navigate("/chat");
                      }}
                      className="flex items-center gap-2 w-full px-3 py-1.5 rounded-full cursor-pointer text-sm hover:bg-background-tertiary/60 transition-colors text-text-primary text-left truncate"
                    >
                      {c.title}
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}

          <div className="px-2 pt-2 pb-2 border-t border-border-secondary">
            <button
              onClick={() => navigate("/settings")}
              className={`flex flex-row items-center gap-3 outline-none no-drag w-full rounded-full px-3 py-2 text-sm font-medium transition-colors ${
                location.pathname === "/settings"
                  ? "bg-background-tertiary text-text-primary"
                  : "text-text-primary hover:bg-background-tertiary/60"
              }`}
            >
              <span className="w-5 h-5 flex-shrink-0 text-text-secondary text-center">⚙️</span>
              {!collapsed && <span className="text-left flex-1 truncate">Settings</span>}
            </button>
            {!collapsed && (
              <button
                className="mt-2 w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary hover:bg-background-tertiary text-text-primary text-sm"
                onClick={() => setTheme(theme === "light" ? "dark" : "light")}
              >
                {theme === "light" ? "🌙 Dark" : "☀️ Light"}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}