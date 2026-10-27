import React, { useEffect, useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "./ui/Dialog";
import { Button } from "./ui/Button";

type Announcement = { id: string; title: string; body: string; minVersion?: string; maxVersion?: string };

function compareVersions(a: string, b: string): number {
  const pa = a.split(".").map((x) => parseInt(x, 10) || 0);
  const pb = b.split(".").map((x) => parseInt(x, 10) || 0);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const x = pa[i] || 0, y = pb[i] || 0;
    if (x !== y) return x - y;
  }
  return 0;
}

const ANNOUNCEMENTS: Announcement[] = [
  { id: "welcome-0.1", title: "Welcome to Gaggle", body: "Gaggle is a local-first AI agent workspace. Explore the sidebar to manage projects, chats, recipes, extensions, and more." },
];

export default function AnnouncementModal() {
  const [queue, setQueue] = useState<Announcement[]>([]);
  const [current, setCurrent] = useState<Announcement | null>(null);

  useEffect(() => {
    const unseen = ANNOUNCEMENTS.filter((a) => !localStorage.getItem(`gaggle.announcement.seen.${a.id}`));
    setQueue(unseen);
    if (unseen.length) setCurrent(unseen[0]);
  }, []);

  const close = () => {
    if (current) localStorage.setItem(`gaggle.announcement.seen.${current.id}`, "1");
    const rest = queue.slice(1);
    setQueue(rest);
    setCurrent(rest.length ? rest[0] : null);
  };

  if (!current) return null;
  return (
    <Dialog open={!!current} onOpenChange={() => close()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{current.title}</DialogTitle>
          <DialogDescription>{current.body}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="default" onClick={close}>Got it</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}