import { useState } from "react";
import { useApp } from "../lib/store";

const NAV: { id: any; label: string; icon: string }[] = [
  { id: "chat", label: "New Chat", icon: "💬" },
  { id: "history", label: "Session History", icon: "🗂" },
  { id: "skills", label: "Skills", icon: "🧩" },
  { id: "recipes", label: "Recipes", icon: "📜" },
  { id: "extensions", label: "Extensions", icon: "🔌" },
  { id: "scheduler", label: "Scheduler", icon: "⏰" },
  { id: "settings", label: "Settings", icon: "⚙️" },
];

export default function Sidebar() {
  const { screen, setScreen, theme, setTheme } = useApp();
  const [collapsed, setCollapsed] = useState(false);
  return (
    <aside className={`sidebar ${collapsed ? "collapsed" : ""}`}>
      <div className="sidebar-header">
        {!collapsed && <span>Gaggle</span>}
        <button className="btn" style={{ padding: "2px 6px" }} onClick={() => setCollapsed((v) => !v)} title="Toggle sidebar">
          {collapsed ? "»" : "«"}
        </button>
      </div>
      <nav className="nav-list">
        {NAV.map((n) => (
          <button
            key={n.id}
            className={`nav-item ${screen === n.id ? "active" : ""}`}
            onClick={() => setScreen(n.id)}
            title={n.label}
          >
            <span className="icon">{n.icon}</span>
            {!collapsed && <span className="label">{n.label}</span>}
          </button>
        ))}
      </nav>
      <div className="sidebar-footer">
        {!collapsed && (
          <button
            className="btn"
            style={{ width: "100%" }}
            onClick={() => setTheme(theme === "light" ? "dark" : "light")}
          >
            {theme === "light" ? "🌙 Dark" : "☀️ Light"}
          </button>
        )}
      </div>
    </aside>
  );
}