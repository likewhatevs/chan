import { describe, expect, test } from "vitest";
import {
  SCREENSAVER_DEFAULT_TIMEOUT_SECS,
  SCREENSAVER_MAX_TIMEOUT_SECS,
  SCREENSAVER_MIN_TIMEOUT_SECS,
  hashPin,
} from "./screensaver";
import clientSource from "../api/client.ts?raw";
import sourceText from "./screensaver.ts?raw";

// `fullstack-a-77` slice 1: SPA client methods + PBKDF2
// PIN-hash helper. State machine + overlay component +
// Settings UI defer to slice 2 / 3.

describe("fullstack-a-77 slice 1: api.screensaver* client methods", () => {
  test("screensaverState hits GET /api/screensaver/state", () => {
    expect(clientSource).toMatch(
      /screensaverState: \(\) =>[\s\S]*?req<\{ enabled: boolean; timeout_secs: number; pin_set: boolean \}>\([\s\S]*?"GET",[\s\S]*?"\/api\/screensaver\/state"/,
    );
  });

  test("screensaverPatch hits PATCH /api/screensaver/state with partial body", () => {
    expect(clientSource).toMatch(
      /screensaverPatch: \(body: \{ enabled\?: boolean; timeout_secs\?: number \}\) =>[\s\S]*?"PATCH",[\s\S]*?"\/api\/screensaver\/state",[\s\S]*?body,/,
    );
  });

  test("screensaverSetPin POSTs the base64 hash", () => {
    expect(clientSource).toMatch(
      /screensaverSetPin: \(hash_b64: string\) =>[\s\S]*?"POST",[\s\S]*?"\/api\/screensaver\/pin",[\s\S]*?\{ hash: hash_b64 \}/,
    );
  });

  test("screensaverClearPin sends DELETE /api/screensaver/pin", () => {
    expect(clientSource).toMatch(
      /screensaverClearPin: \(\) =>[\s\S]*?"DELETE",[\s\S]*?"\/api\/screensaver\/pin"/,
    );
  });

  test("screensaverVerify returns { verified } from POST /verify", () => {
    expect(clientSource).toMatch(
      /screensaverVerify: \(hash_b64: string\) =>[\s\S]*?req<\{ verified: boolean \}>\([\s\S]*?"POST",[\s\S]*?"\/api\/screensaver\/verify",[\s\S]*?\{ hash: hash_b64 \}/,
    );
  });

  test("doc-comment cross-references systacean-40 + hash-on-wire shape", () => {
    expect(clientSource).toMatch(/`fullstack-a-77`/);
    expect(clientSource).toMatch(/`systacean-40`/);
    expect(clientSource).toMatch(/pin_set: bool/);
  });
});

describe("fullstack-a-77 slice 1: PBKDF2 hashPin helper", () => {
  test("hashPin produces a deterministic base64 digest for same inputs", async () => {
    const a = await hashPin("1234", "/tmp/drive-a");
    const b = await hashPin("1234", "/tmp/drive-a");
    expect(a).toBe(b);
    // Base64 of 32 bytes = 44 chars including padding.
    expect(a).toHaveLength(44);
  });

  test("different drive salts yield different hashes for the same PIN", async () => {
    const a = await hashPin("1234", "/tmp/drive-a");
    const b = await hashPin("1234", "/tmp/drive-b");
    expect(a).not.toBe(b);
  });

  test("different PINs yield different hashes for the same salt", async () => {
    const a = await hashPin("1234", "/tmp/drive-a");
    const b = await hashPin("1235", "/tmp/drive-a");
    expect(a).not.toBe(b);
  });

  test("empty drive salt falls back to a fixed default + still hashes", async () => {
    const hash = await hashPin("1234", "");
    expect(hash).toHaveLength(44);
  });
});

describe("fullstack-a-77 slice 1: timeout constants", () => {
  test("default matches the chan-drive 300s default", () => {
    expect(SCREENSAVER_DEFAULT_TIMEOUT_SECS).toBe(300);
  });

  test("min + max bracket the configurable range", () => {
    expect(SCREENSAVER_MIN_TIMEOUT_SECS).toBe(30);
    expect(SCREENSAVER_MAX_TIMEOUT_SECS).toBe(4 * 60 * 60);
  });
});

describe("fullstack-a-77 slice 1: rationale documented in source", () => {
  test("module doc-comment cites the threat-model + iteration choice", () => {
    expect(sourceText).toMatch(/local-only/);
    expect(sourceText).toMatch(/PBKDF2 \+[\s\S]{1,30}SHA-256/);
    expect(sourceText).toMatch(/100_000/);
  });

  test("PBKDF2_ITERATIONS constant set to OWASP minimum", () => {
    expect(sourceText).toMatch(/const PBKDF2_ITERATIONS = 100_000;/);
  });
});
