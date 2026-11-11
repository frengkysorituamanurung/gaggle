import { useEffect, useRef } from "react";

export function useAutoSubmit(callback: () => void, deps: unknown[] = []) {
  const cbRef = useRef(callback);
  cbRef.current = callback;
  useEffect(() => {
    // Auto-submit on mount if provided
    // (kept minimal; callers control when to invoke)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return cbRef;
}

export default useAutoSubmit;