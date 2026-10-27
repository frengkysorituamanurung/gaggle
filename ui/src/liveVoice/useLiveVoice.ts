import React from "react";

export type LiveVoiceController = {
  activeSessionId: string | null;
  listening: boolean;
  transcript: string;
  supported: boolean;
  start: (sessionId: string) => void;
  stop: () => void;
};

export function useLiveVoice(): LiveVoiceController {
  const [activeSessionId, setActiveSessionId] = React.useState<string | null>(null);
  const [listening, setListening] = React.useState(false);
  const [transcript, setTranscript] = React.useState("");
  const recRef = React.useRef<any>(null);

  const supported = typeof window !== "undefined" && ("SpeechRecognition" in window || "webkitSpeechRecognition" in window);

  const start = React.useCallback((sessionId: string) => {
    setActiveSessionId(sessionId);
    if (!supported) return;
    const SR = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
    const rec = new SR();
    rec.continuous = true;
    rec.interimResults = true;
    rec.onresult = (e: any) => {
      let t = "";
      for (let i = 0; i < e.results.length; i++) t += e.results[i][0].transcript;
      setTranscript(t);
    };
    rec.onend = () => setListening(false);
    rec.start();
    setListening(true);
    recRef.current = rec;
  }, [supported]);

  const stop = React.useCallback(() => {
    recRef.current?.stop();
    setListening(false);
    setActiveSessionId(null);
  }, []);

  return { activeSessionId, listening, transcript, supported, start, stop };
}