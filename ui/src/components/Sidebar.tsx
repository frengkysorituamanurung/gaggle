import { useState } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { useApp } from "../lib/store";

const NAV: { path: string; label: string; icon: string }[] = [
  { path: "/chat", label: "New Chat", icon: "💬" },
  { path: "/history", label: "Session History", icon: "🗂" },
  { path: "/skills", label: "Skills", icon: "🧩" },
  { path: "/recipes", label: "Recipes", icon: "📜" },
  { path: "/extensions", label: "Extensions", icon: "🔌" },
  { path: "/scheduler", label: "Scheduler", icon: "⏰" },
  { path: "/settings", label: "Settings", icon: "⚙️" },
];

export default function Sidebar() {
  const navigate = useNavigate();
  const location = useLocation();
  const { theme, setTheme } = useApp();
  const [collapsed, setCollapsed] = useState(false);
  return (
    <aside
      className={`flex flex-col shrink-0 bg-surface border border-border rounded-xl overflow-hidden transition-all ${
        collapsed ? "w-14" : "w-60"
      }`}
    >
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        {!collapsed && <span className="font-semibold text-text">Gaggle</span>}
        <button
          className="px-2 py-1 text-xs rounded-md border border-border bg-background hover:bg-surface-2 text-text"
          onClick={() => setCollapsed((v) => !v)}
          title="Toggle sidebar"
        >
          {collapsed ? "»" : "«"}
        </button>
      </div>
      <nav className="flex-1 overflow-y-auto p-2 space-y-1">
        {NAV.map((n) => {
          const active = location.pathname === n.path;
          return (
            <button
              key={n.path}
              onClick={() => navigate(n.path)}
              title={n.label}
              className={`flex items-center gap-3 w-full px-3 py-2 rounded-full text-sm font-medium transition-colors ${
                active ? "bg-surface-2 text-text" : "text-text hover:bg-surface-2/60"
              }`}
            >
              <span className="w-5 text-center">{n.icon}</span>
              {!collapsed && <span className="truncate">{n.label}</span>}
            </button>
          );
        })}
      </nav>
      {!collapsed && (
        <div className="p-3 border-t border-border">
          <button
            className="w-full px-3 py-2 rounded-md border border-border bg-background hover:bg-surface-2 text-text text-sm"
            onClick={() => setTheme(theme === "light" ? "dark" : "light")}
          >
            {theme === "light" ? "🌙 Dark" : "☀️ Light"}
          </button>
        </div>
      )}
    </aside>
  );
}