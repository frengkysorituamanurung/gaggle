const cache = new Map<string, unknown>();
export const toolsCache = { get: (k: string) => cache.get(k), set: (k: string, v: unknown) => cache.set(k, v), clear: () => cache.clear() };