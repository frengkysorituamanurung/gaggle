import React, { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/Card";
import { Button } from "./ui/Button";

interface Announcement {
  id: string;
  title: string;
  body: string;
  date: string;
}

const ANNOUNCEMENTS: Announcement[] = [
  {
    id: "welcome",
    title: "Welcome to Gaggle",
    body: "Gaggle is your local AI agent workspace. Configure a provider in Settings to get started.",
    date: "2026-10-29",
  },
];

export default function AnnouncementsModal() {
  const [visible, setVisible] = useState(false);
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());

  useEffect(() => {
    const stored = localStorage.getItem("gaggle-dismissed-announcements");
    if (stored) {
      try {
        setDismissed(new Set(JSON.parse(stored)));
      } catch {
        // ignore
      }
    }
    // Show if there are undismissed announcements
    const hasUndismissed = ANNOUNCEMENTS.some((a) => !dismissed.has(a.id));
    if (hasUndismissed) {
      setVisible(true);
    }
  }, [dismissed]);

  const dismiss = (id: string) => {
    const next = new Set(dismissed);
    next.add(id);
    setDismissed(next);
    localStorage.setItem("gaggle-dismissed-announcements", JSON.stringify([...next]));
  };

  const dismissAll = () => {
    const next = new Set(ANNOUNCEMENTS.map((a) => a.id));
    setDismissed(next);
    localStorage.setItem("gaggle-dismissed-announcements", JSON.stringify([...next]));
    setVisible(false);
  };

  if (!visible) return null;

  const undismissed = ANNOUNCEMENTS.filter((a) => !dismissed.has(a.id));
  if (undismissed.length === 0) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <Card className="w-full max-w-md mx-4">
        <CardHeader>
          <CardTitle className="text-text-primary text-base">
            {undismissed[0].title}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-text-secondary text-sm mb-4">{undismissed[0].body}</p>
          <div className="flex justify-end gap-2">
            <Button size="sm" variant="secondary" onClick={() => dismiss(undismissed[0].id)}>
              Dismiss
            </Button>
            <Button size="sm" onClick={dismissAll}>
              Got it
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}