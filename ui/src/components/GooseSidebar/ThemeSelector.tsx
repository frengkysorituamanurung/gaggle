import React from "react";
import { useApp } from "../../lib/store";
import { cn } from "../../lib/utils";

export default function ThemeSelector() {
  const { theme, setTheme } = useApp();
  const options: { id: "light" | "dark" | "aura"; label: string }[] = [
    { id: "light", label: "Light" },
    { id: "dark", label: "Dark" },
    { id: "aura", label: "Aura" },
  ];
  return (
    <div className="flex gap-1">
      {options.map((o) => (
        <button
          key={o.id}
          onClick={() => setTheme(o.id)}
          className={cn(
            "px-2 py-1 rounded-md text-xs border border-border-primary",
            theme === o.id ? "bg-background-inverse text-text-inverse" : "bg-background-primary text-text-primary hover:bg-background-secondary"
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}