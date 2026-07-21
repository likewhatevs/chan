// @vitest-environment jsdom

import { afterEach, describe, expect, test } from "vitest";
import { chanFetch, gatewayCsrfHeaderPairs, setFetchImpl } from "./transport";

afterEach(() => {
  setFetchImpl(null);
  // `Secure` is required: the `__Host-` prefix mandates it, and jsdom's cookie
  // jar rejects a `__Host-` cookie set without it, so the read would see nothing.
  document.cookie = "__Host-devserver_csrf=; Max-Age=0; path=/; Secure";
});

describe("gateway CSRF", () => {
  test("chanFetch mirrors the readable gateway csrf cookie on unsafe requests", async () => {
    document.cookie = "__Host-devserver_csrf=csrf-token; path=/; Secure";
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
    document.cookie = "__Host-devserver_csrf=csrf-token; path=/; Secure";
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

describe("gatewayCsrfHeaderPairs", () => {
  test("carries the cookie mirror for unsafe methods only", () => {
    document.cookie = "__Host-devserver_csrf=csrf-token; path=/; Secure";

    expect(gatewayCsrfHeaderPairs("POST")).toEqual([
      ["x-chan-csrf", "csrf-token"],
    ]);
    expect(gatewayCsrfHeaderPairs("delete")).toEqual([
      ["x-chan-csrf", "csrf-token"],
    ]);
    expect(gatewayCsrfHeaderPairs("GET")).toEqual([]);
  });

  test("is empty without the cookie (loopback)", () => {
    expect(gatewayCsrfHeaderPairs("POST")).toEqual([]);
  });
});
