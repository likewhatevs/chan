// @vitest-environment jsdom

import { describe, expect, test } from "vitest";
import { __testIsTransientBootstrapError } from "./store.svelte";
import { ApiError } from "../api/errors";

// bug 8 (desktop auto-reload + hang on loading): when WKWebView
// recycles a drive window's web-content process and the SPA reloads
// while the embedded loopback server is briefly unreachable, bootstrap
// must retry transient failures instead of sticking on "loading...".
// These pin which failures count as transient (retry) vs terminal
// (surface immediately).
describe("isTransientBootstrapError", () => {
  test("connection-refused / dropped-socket fetch (bare Error) is transient", () => {
    // fetch() to a refused loopback socket rejects with a TypeError.
    expect(__testIsTransientBootstrapError(new TypeError("Failed to fetch"))).toBe(true);
    expect(__testIsTransientBootstrapError(new Error("network down"))).toBe(true);
  });

  test("our transport timeout (ApiError status 0) is transient", () => {
    expect(__testIsTransientBootstrapError(new ApiError(0, "request timed out"))).toBe(true);
  });

  test("5xx from a still-spinning-up server is transient", () => {
    expect(__testIsTransientBootstrapError(new ApiError(502, "bad gateway"))).toBe(true);
    expect(__testIsTransientBootstrapError(new ApiError(503, "unavailable"))).toBe(true);
    expect(__testIsTransientBootstrapError(new ApiError(504, "gateway timeout"))).toBe(true);
  });

  test("401 (missing token) is NOT transient: must surface the overlay", () => {
    expect(__testIsTransientBootstrapError(new ApiError(401, "unauthorized"))).toBe(false);
  });

  test("404 / other 4xx is NOT transient: a real error", () => {
    expect(__testIsTransientBootstrapError(new ApiError(404, "not found"))).toBe(false);
    expect(__testIsTransientBootstrapError(new ApiError(409, "conflict"))).toBe(false);
  });

  test("a non-Error throwable is NOT transient", () => {
    expect(__testIsTransientBootstrapError("boom")).toBe(false);
    expect(__testIsTransientBootstrapError(undefined)).toBe(false);
  });
});
