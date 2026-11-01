import { useState, useCallback } from "react";

export function useAudioRecorder(opts?: { onTranscription?: (t: string) => void; onError?: (m: string) => void }) {
  const [isRecording, setIsRecording] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const startRecording = useCallback(async () => {
    try {
      setIsRecording(true);
    } catch (e) {
      opts?.onError?.(String(e));
    }
  }, [opts]);
  const stopRecording = useCallback(async () => {
    setIsRecording(false);
    setIsTranscribing(true);
    try {
      // Transcription not wired to backend; emit empty result.
      opts?.onTranscription?.("");
    } catch (e) {
      opts?.onError?.(String(e));
    } finally {
      setIsTranscribing(false);
    }
  }, [opts]);
  return { isEnabled: true, dictationProvider: "browser", isRecording, isTranscribing, startRecording, stopRecording };
}

export default useAudioRecorder;