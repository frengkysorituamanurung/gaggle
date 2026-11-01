import { useRef, useEffect } from "react";

export function useFocusOnTyping<T extends HTMLElement = HTMLTextAreaElement>() {
  const ref = useRef<T | null>(null);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key.length === 1 && !e.metaKey && !e.ctrlKey && !e.altKey) {
        ref.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  return ref;
}

export default useFocusOnTyping;