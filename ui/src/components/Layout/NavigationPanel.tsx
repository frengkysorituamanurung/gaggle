import React, { useEffect, useState } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { ChevronDown, ChevronRight, Settings } from "lucide-react";
import { useApp } from "../../lib/store";
import { api } from "../../lib/api";
import type * as T from "../../types";
import { cn } from "../../lib/utils";
import { defineMessages, useIntl } from "../../i18n";

import {
  MessageSquarePlus,
  BookOpen,
  Puzzle,
  Plug,
  Clock,
  History,
  Brain,
  Shield,
  FolderTree,
  Terminal,
  AppWindow,
} from "lucide-react";

const navMessages = defineMessages({
  home: { id: "navigation.home", defaultMessage: "Home" },
  recipes: { id: "navigation.recipes", defaultMessage: "Recipes" },
  skills: { id: "navigation.skills", defaultMessage: "Skills" },
  apps: { id: "navigation.apps", defaultMessage: "Apps" },
  extensions: { id: "navigation.extensions", defaultMessage: "Extensions" },
  scheduler: { id: "navigation.scheduler", defaultMessage: "Scheduler" },
  memories: { id: "navigation.memories", defaultMessage: "Memories" },
  permissions: { id: "navigation.permissions", defaultMessage: "Permissions" },
  workspace: { id: "navigation.workspace", defaultMessage: "Workspace" },
  terminal: { id: "navigation.terminal", defaultMessage: "Terminal" },
  sessionHistory: { id: "navigation.sessionHistory", defaultMessage: "Session History" },
  chats: { id: "navigation.chats", defaultMessage: "Chats" },
  noRecentChats: { id: "navigation.noRecentChats", defaultMessage: "No recent chats" },
  settings: { id: "navigation.settings", defaultMessage: "Settings" },
});

const NAV = [
  { path: "/", label: navMessages.home, icon: MessageSquarePlus },
  { path: "/recipes", label: navMessages.recipes, icon: BookOpen },
  { path: "/skills", label: navMessages.skills, icon: Puzzle },
  { path: "/apps", label: navMessages.apps, icon: AppWindow },
  { path: "/extensions", label: navMessages.extensions, icon: Plug },
  { path: "/scheduler", label: navMessages.scheduler, icon: Clock },
  { path: "/memories", label: navMessages.memories, icon: Brain },
  { path: "/permissions", label: navMessages.permissions, icon: Shield },
  { path: "/workspace", label: navMessages.workspace, icon: FolderTree },
  { path: "/terminal", label: navMessages.terminal, icon: Terminal },
  { path: "/history", label: navMessages.sessionHistory, icon: History },
];

export const Navigation: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const { theme, setTheme, projectId, setChatId } = useApp();
  const intl = useIntl();
  const [chats, setChats] = useState<T.Chat[]>([]);
  const [isChatsExpanded, setIsChatsExpanded] = useState(true);

  useEffect(() => {
    if (!projectId) { setChats([]); return; }
    api.projectChats(projectId).then(setChats).catch(() => {});
  }, [projectId]);

  const isActive = (path: string) => location.pathname === path;

  return (
    <div className="bg-background-primary outline-none flex flex-col h-full">
      <div className="h-[48px] no-drag" />

      <div className="px-2 flex flex-col gap-0.5">
        {NAV.map((n) => {
          const Icon = n.icon;
          const active = isActive(n.path);
          return (
            <button
              key={n.path}
              onClick={() => navigate(n.path)}
              className={cn(
                "flex flex-row items-center gap-3 outline-none no-drag w-full rounded-full px-3 py-2 text-sm font-medium transition-colors",
                active ? "bg-background-tertiary text-text-primary" : "text-text-primary hover:bg-background-tertiary/60"
              )}
            >
              <Icon className="w-5 h-5 flex-shrink-0 text-text-secondary" />
              <span className="text-left flex-1 truncate">{intl.formatMessage(n.label)}</span>
            </button>
          );
        })}
      </div>

      <div className="flex-1 min-h-0 flex flex-col mt-3">
        <button
          onClick={() => setIsChatsExpanded((v) => !v)}
          className="flex items-center gap-1 px-4 py-1 text-xs font-semibold uppercase tracking-wider text-text-secondary hover:text-text-primary transition-colors self-start"
        >
          {isChatsExpanded ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
          <span>{intl.formatMessage(navMessages.chats)}</span>
        </button>
        {isChatsExpanded && (
          <div className="flex-1 min-h-0 overflow-y-auto px-2 pb-2 mt-1 sidebar-scrollbar">
            {chats.length === 0 ? (
              <div className="px-3 py-2 text-xs text-text-secondary">{intl.formatMessage(navMessages.noRecentChats)}</div>
            ) : (
              chats.map((c) => (
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
              ))
            )}
          </div>
        )}
      </div>

      <div className="px-2 pt-2 pb-2 border-t border-border-secondary">
        <button
          onClick={() => navigate("/settings")}
          className={cn(
            "flex flex-row items-center gap-3 outline-none no-drag w-full rounded-full px-3 py-2 text-sm font-medium transition-colors",
            isActive("/settings") ? "bg-background-tertiary text-text-primary" : "text-text-primary hover:bg-background-tertiary/60"
          )}
        >
          <Settings className="w-5 h-5 flex-shrink-0 text-text-secondary" />
          <span className="text-left flex-1 truncate">{intl.formatMessage(navMessages.settings)}</span>
        </button>
        <button
          className="mt-2 w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary hover:bg-background-tertiary text-text-primary text-sm"
          onClick={() => setTheme(theme === "light" ? "dark" : "light")}
        >
          {theme === "light" ? "🌙 Dark" : "☀️ Light"}
        </button>
      </div>
    </div>
  );
};