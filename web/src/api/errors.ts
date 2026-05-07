// Shared error type for any API call, regardless of transport.
//
// Lives in its own module so the transport layer can throw it without
// pulling in the rest of `client.ts` and creating a cycle.

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}
