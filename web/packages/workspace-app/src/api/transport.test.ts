// @vitest-environment jsdom

import { afterEach, describe, expect, test } from "vitest";
import { chanFetch, setFetchImpl } from "./transport";

afterEach(() => {
  setFetchImpl(null);
  document.cookie = "devserver_csrf=; Max-Age=0; path=/";
});

describe("gateway CSRF", () => {
  test("chanFetch mirrors the readable gateway csrf cookie on unsafe requests", async () => {
    document.cookie = "devserver_csrf=csrf-token; path=/";
    let seen: RequestInit | undefined;
    setFetchImpl(async (_input, init) => {
      seen = init;
      return new Response("", { status: 200 });
    });

    await chanFetch("/api/session?w=w-test", {
      method: "PUT",
      headers: { "content-type": "application/json" },
      body: "{}",
    });

    const headers = seen?.headers as Record<string, string>;
    expect(headers["content-type"]).toBe("application/json");
    expect(headers["x-chan-csrf"]).toBe("csrf-token");
  });

  test("chanFetch leaves safe requests without the csrf mirror", async () => {
    document.cookie = "devserver_csrf=csrf-token; path=/";
    let seen: RequestInit | undefined;
    setFetchImpl(async (_input, init) => {
      seen = init;
      return new Response("", { status: 200 });
    });

    await chanFetch("/api/session?w=w-test", {
      method: "GET",
      headers: { authorization: "Bearer tok" },
    });

    const headers = seen?.headers as Record<string, string>;
    expect(headers.authorization).toBe("Bearer tok");
    expect(headers["x-chan-csrf"]).toBeUndefined();
  });
});
