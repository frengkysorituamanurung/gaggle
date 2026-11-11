import React from "react";
import { Button } from "./ui/Button";
import { AudioLines, MicOff } from "lucide-react";
import type { LiveVoiceController } from "../liveVoice/useLiveVoice";

export default function LiveVoiceButton({ controller, sessionId }: { controller?: LiveVoiceController; sessionId?: string }) {
  const listening = controller?.listening ?? false;
  const supported = controller?.supported ?? false;
  const active = controller?.activeSessionId === sessionId;
  return (
    <Button
      size="sm"
      variant={listening && active ? "default" : "secondary"}
      title={supported ? "Live voice" : "Live voice not supported"}
      disabled={!supported}
      onClick={() => {
        if (!controller || !sessionId) return;
        if (listening && active) controller.stop();
        else controller.start(sessionId);
      }}
    >
      {listening && active ? <MicOff className="w-4 h-4" /> : <AudioLines className="w-4 h-4" />}
    </Button>
  );
}