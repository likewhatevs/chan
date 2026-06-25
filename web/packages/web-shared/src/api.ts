// Shared fetch wrapper for the SPA embedded by identity-service.
// credentials: include so the id_session cookie set by identity-service
// is sent on every /api call.

export class HttpError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

export async function request<T>(
  input: string,
  init?: RequestInit,
): Promise<T> {
  const res = await fetch(input, { credentials: "include", ...init });
  if (res.status === 204) return undefined as T;
  const body = res.headers.get("content-type")?.includes("application/json")
    ? await res.json()
    : await res.text();
  if (!res.ok) {
    const msg = typeof body === "string"
      ? body
      : (body?.error ?? res.statusText);
    throw new HttpError(res.status, String(msg));
  }
  return body as T;
}
