import React, { useState, useEffect } from "react";
import { useApp } from "../../lib/store";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card";
import { Button } from "../ui/Button";

const THEMES = [
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
  { id: "system", label: "System" },
];

export default function ThemeSettingsSection() {
  const { theme, setTheme } = useApp();
  const [selected, setSelected] = useState(theme || "system");

  useEffect(() => {
    setSelected(theme || "system");
  }, [theme]);

  const apply = (t: string) => {
    setSelected(t);
    setTheme(t);
    document.documentElement.setAttribute("data-theme", t === "system" ? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light") : t);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-text-primary text-base">Theme</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex gap-2">
          {THEMES.map((t) => (
            <Button
              key={t.id}
              size="sm"
              variant={selected === t.id ? "default" : "secondary"}
              onClick={() => apply(t.id)}
            >
              {t.label}
            </Button>
          ))}
        </div>
        <p className="text-text-secondary text-xs mt-2">
          Choose your preferred color theme. System follows OS settings.
        </p>
      </CardContent>
    </Card>
  );
}