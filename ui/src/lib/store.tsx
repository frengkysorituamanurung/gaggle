import React, { createContext, useContext, useState, useEffect } from "react";

export type Screen =
  | "chat"
  | "history"
  | "skills"
  | "recipes"
  | "extensions"
  | "scheduler"
  | "settings";

import { applyThemeTokens } from "../theme/theme-tokens";
type Theme = "light" | "dark" | "aura";

type AppState = {
  theme: Theme;
  setTheme: (t: Theme) => void;
  screen: Screen;
  setScreen: (s: Screen) => void;
  projectId: string | null;
  setProjectId: (id: string | null) => void;
  chatId: string | null;
  setChatId: (id: string | null) => void;
};

const Ctx = createContext<AppState | null>(null);

export function AppProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(() => {
    const saved = localStorage.getItem("gaggle.theme");
    if (saved === "light" || saved === "dark") return saved;
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  });
  const [screen, setScreen] = useState<Screen>("chat");
  const [projectId, setProjectId] = useState<string | null>(null);
  const [chatId, setChatId] = useState<string | null>(null);

  useEffect(() => {
    applyThemeTokens(theme);
    localStorage.setItem("gaggle.theme", theme);
  }, [theme]);

  const setTheme = (t: Theme) => setThemeState(t);

  return (
    <Ctx.Provider
      value={{ theme, setTheme, screen, setScreen, projectId, setProjectId, chatId, setChatId }}
    >
      {children}
    </Ctx.Provider>
  );
}

export function useApp() {
  const v = useContext(Ctx);
  if (!v) throw new Error("useApp outside provider");
  return v;
}