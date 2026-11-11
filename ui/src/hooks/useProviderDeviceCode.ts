import { useState, useCallback } from "react";

export function useProviderDeviceCode() {
  const [code, setCode] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const start = useCallback(async (providerId: string) => {
    setLoading(true);
    try {
      // Device code flow not wired; placeholder.
      setCode(null);
    } finally {
      setLoading(false);
    }
  }, []);
  const cancel = useCallback(() => { setCode(null); }, []);
  return { code, loading, start, cancel };
}

export default useProviderDeviceCode;