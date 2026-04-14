// In dev, Vite proxies /api/* to the backend.
// In production, the frontend is served from the backend, so we hit the same origin.
const BASE = import.meta.env.DEV ? "/api" : "";

export async function apiFetch<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`);
  if (!res.ok) throw new Error(`API error: ${res.status}`);
  return res.json() as Promise<T>;
}
