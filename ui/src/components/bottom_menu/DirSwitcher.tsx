import React, { useState } from "react";
import { Button } from "../ui/Button";
import { FolderOpen } from "lucide-react";

export function DirSwitcher({
  currentDir,
  onSwitch,
}: {
  currentDir?: string | null;
  onSwitch?: (dir: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [value, setValue] = useState("");
  return (
    <div className="relative">
      <Button variant="ghost" size="sm" onClick={() => setOpen((v) => !v)} title="Switch working directory">
        <FolderOpen className="w-4 h-4" />
        {currentDir && <span className="ml-1 text-xs truncate max-w-[120px]">{currentDir}</span>}
      </Button>
      {open && (
        <div className="absolute bottom-full mb-2 z-50 w-64 rounded-md border border-border-primary bg-background-primary p-2 shadow-lg">
          <input
            className="w-full px-2 py-1 rounded-md border border-border-primary bg-background-secondary text-text-primary text-sm mb-2"
            placeholder="Enter directory path"
            value={value}
            onChange={(e) => setValue(e.target.value)}
          />
          <div className="flex justify-end gap-2">
            <Button size="sm" variant="secondary" onClick={() => setOpen(false)}>Cancel</Button>
            <Button size="sm" variant="default" onClick={() => { onSwitch?.(value); setOpen(false); setValue(""); }}>Switch</Button>
          </div>
        </div>
      )}
    </div>
  );
}

export default DirSwitcher;