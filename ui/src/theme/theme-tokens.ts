// Ported from goose/ui/desktop/src/theme/theme-tokens.ts
// Static token definitions for Gaggle (web). Applied to :root via applyThemeTokens.

export type ThemeId = "light" | "dark" | "aura";

const baseTokens: Record<string, string> = {
  "--font-sans": "'Cash Sans', sans-serif",
  "--font-mono": "monospace",
  "--font-weight-normal": "400",
  "--font-weight-medium": "500",
  "--font-weight-semibold": "600",
  "--font-weight-bold": "700",
  "--font-text-xs-size": "0.75rem",
  "--font-text-sm-size": "0.875rem",
  "--font-text-md-size": "1rem",
  "--font-text-lg-size": "1.125rem",
  "--font-heading-xs-size": "1rem",
  "--font-heading-sm-size": "1.125rem",
  "--font-heading-md-size": "1.25rem",
  "--font-heading-lg-size": "1.5rem",
  "--font-heading-xl-size": "1.875rem",
  "--font-heading-2xl-size": "2.25rem",
  "--font-heading-3xl-size": "3rem",
  "--font-text-xs-line-height": "1rem",
  "--font-text-sm-line-height": "1.25rem",
  "--font-text-md-line-height": "1.5rem",
  "--font-text-lg-line-height": "1.75rem",
  "--font-heading-xs-line-height": "1.5rem",
  "--font-heading-sm-line-height": "1.75rem",
  "--font-heading-md-line-height": "1.75rem",
  "--font-heading-lg-line-height": "2rem",
  "--font-heading-xl-line-height": "2.25rem",
  "--font-heading-2xl-line-height": "2.5rem",
  "--font-heading-3xl-line-height": "3.5rem",
  "--border-radius-xs": "2px",
  "--border-radius-sm": "4px",
  "--border-radius-md": "8px",
  "--border-radius-lg": "12px",
  "--border-radius-xl": "16px",
  "--border-radius-full": "9999px",
  "--border-width-regular": "1px",
};

const lightColorTokens: Record<string, string> = {
  "--color-background-primary": "#ffffff",
  "--color-background-secondary": "#f4f6f7",
  "--color-background-tertiary": "#e3e6ea",
  "--color-background-inverse": "#000000",
  "--color-background-ghost": "transparent",
  "--color-background-info": "#5c98f9",
  "--color-background-danger": "#f94b4b",
  "--color-background-success": "#91cb80",
  "--color-background-warning": "#fbcd44",
  "--color-background-disabled": "#e3e6ea",
  "--color-text-primary": "#3f434b",
  "--color-text-secondary": "#878787",
  "--color-text-tertiary": "#a7b0b9",
  "--color-text-inverse": "#ffffff",
  "--color-text-ghost": "#878787",
  "--color-text-info": "#5c98f9",
  "--color-text-danger": "#f94b4b",
  "--color-text-success": "#91cb80",
  "--color-text-warning": "#fbcd44",
  "--color-text-disabled": "#cbd1d6",
  "--color-border-primary": "#e3e6ea",
  "--color-border-secondary": "#e3e6ea",
  "--color-border-tertiary": "#cbd1d6",
  "--color-border-inverse": "#000000",
  "--color-border-ghost": "transparent",
  "--color-border-info": "#5c98f9",
  "--color-border-danger": "#f94b4b",
  "--color-border-success": "#91cb80",
  "--color-border-warning": "#fbcd44",
  "--color-border-disabled": "#e3e6ea",
  "--color-ring-primary": "#e3e6ea",
  "--color-ring-secondary": "#cbd1d6",
  "--color-ring-inverse": "#ffffff",
  "--color-ring-info": "#5c98f9",
  "--color-ring-danger": "#f94b4b",
  "--color-ring-success": "#91cb80",
  "--color-ring-warning": "#fbcd44",
  "--shadow-hairline": "0 0 0 1px rgba(0, 0, 0, 0.05)",
  "--shadow-sm": "0 1px 2px 0 rgba(0, 0, 0, 0.05)",
  "--shadow-md": "0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -2px rgba(0, 0, 0, 0.1)",
  "--shadow-lg": "0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -4px rgba(0, 0, 0, 0.1)",
};

const darkColorTokens: Record<string, string> = {
  "--color-background-primary": "#22252a",
  "--color-background-secondary": "#3f434b",
  "--color-background-tertiary": "#474e57",
  "--color-background-inverse": "#cbd1d6",
  "--color-background-ghost": "transparent",
  "--color-background-info": "#7cacff",
  "--color-background-danger": "#ff6b6b",
  "--color-background-success": "#a3d795",
  "--color-background-warning": "#ffd966",
  "--color-background-disabled": "#474e57",
  "--color-text-primary": "#ffffff",
  "--color-text-secondary": "#878787",
  "--color-text-tertiary": "#606c7a",
  "--color-text-inverse": "#000000",
  "--color-text-ghost": "#878787",
  "--color-text-info": "#7cacff",
  "--color-text-danger": "#ff6b6b",
  "--color-text-success": "#a3d795",
  "--color-text-warning": "#ffd966",
  "--color-text-disabled": "#525b68",
  "--color-border-primary": "#3f434b",
  "--color-border-secondary": "#525b68",
  "--color-border-tertiary": "#474e57",
  "--color-border-inverse": "#ffffff",
  "--color-border-ghost": "transparent",
  "--color-border-info": "#7cacff",
  "--color-border-danger": "#ff6b6b",
  "--color-border-success": "#a3d795",
  "--color-border-warning": "#ffd966",
  "--color-border-disabled": "#3f434b",
  "--color-ring-primary": "#525b68",
  "--color-ring-secondary": "#474e57",
  "--color-ring-inverse": "#000000",
  "--color-ring-info": "#7cacff",
  "--color-ring-danger": "#ff6b6b",
  "--color-ring-success": "#a3d795",
  "--color-ring-warning": "#ffd966",
  "--shadow-hairline": "0 0 0 1px rgba(0, 0, 0, 0.2)",
  "--shadow-sm": "0 1px 2px 0 rgba(0, 0, 0, 0.2)",
  "--shadow-md": "0 4px 6px -1px rgba(0, 0, 0, 0.3), 0 2px 4px -2px rgba(0, 0, 0, 0.2)",
  "--shadow-lg": "0 10px 15px -3px rgba(0, 0, 0, 0.3), 0 4px 6px -4px rgba(0, 0, 0, 0.2)",
};

const auraColorTokens: Record<string, string> = {
  "--color-background-primary": "#15141b",
  "--color-background-secondary": "#1a1921",
  "--color-background-tertiary": "#201e2b",
  "--color-background-inverse": "#a277ff",
  "--color-background-ghost": "transparent",
  "--color-background-info": "#82e2ff",
  "--color-background-danger": "#ff6767",
  "--color-background-success": "#61ffca",
  "--color-background-warning": "#ffca85",
  "--color-background-disabled": "#25232f",
  "--color-text-primary": "#edecee",
  "--color-text-secondary": "#8a8282",
  "--color-text-tertiary": "#6d6d6d",
  "--color-text-inverse": "#15141b",
  "--color-text-ghost": "#8a8282",
  "--color-text-info": "#82e2ff",
  "--color-text-danger": "#ff6767",
  "--color-text-success": "#61ffca",
  "--color-text-warning": "#ffca85",
  "--color-text-disabled": "#525b68",
  "--color-border-primary": "#2d2b38",
  "--color-border-secondary": "#47415a",
  "--color-border-tertiary": "#4e496c",
  "--color-border-inverse": "#edecee",
  "--color-border-ghost": "transparent",
  "--color-border-info": "#82e2ff",
  "--color-border-danger": "#ff6767",
  "--color-border-success": "#61ffca",
  "--color-border-warning": "#ffca85",
  "--color-border-disabled": "#2d2b38",
  "--color-ring-primary": "#47415a",
  "--color-ring-secondary": "#2d2b38",
  "--color-ring-inverse": "#15141b",
  "--color-ring-info": "#82e2ff",
  "--color-ring-danger": "#ff6767",
  "--color-ring-success": "#61ffca",
  "--color-ring-warning": "#ffca85",
  "--shadow-hairline": "0 0 0 1px rgba(0, 0, 0, 0.2)",
  "--shadow-sm": "0 1px 2px 0 rgba(0, 0, 0, 0.2)",
  "--shadow-md": "0 4px 6px -1px rgba(0, 0, 0, 0.3), 0 2px 4px -2px rgba(0, 0, 0, 0.2)",
  "--shadow-lg": "0 10px 15px -3px rgba(0, 0, 0, 0.3), 0 4px 6px -4px rgba(0, 0, 0, 0.2)",
  "--font-sans": "monospace",
  "--font-mono": "monospace",
};

const colorSets: Record<ThemeId, Record<string, string>> = {
  light: lightColorTokens,
  dark: darkColorTokens,
  aura: auraColorTokens,
};

export const themes: Record<ThemeId, { tokens: Record<string, string> }> = {
  light: { tokens: { ...baseTokens, ...lightColorTokens } },
  dark: { tokens: { ...baseTokens, ...darkColorTokens } },
  aura: { tokens: { ...baseTokens, ...auraColorTokens } },
};

export function getResolvedTheme(): ThemeId {
  const stored = localStorage.getItem("gaggle.theme");
  if (stored === "light" || stored === "dark" || stored === "aura") return stored as ThemeId;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function applyThemeTokens(theme: ThemeId) {
  const tokens = themes[theme].tokens;
  const root = document.documentElement;
  Object.entries(tokens).forEach(([k, v]) => root.style.setProperty(k, v));
  root.classList.toggle("dark", theme === "dark" || theme === "aura");
  root.classList.toggle("light", theme === "light");
  root.setAttribute("data-theme", theme === "aura" ? "dark" : theme);
  root.style.colorScheme = theme === "light" ? "light" : "dark";
}

export function buildMcpHostStyles() {
  return {};
}