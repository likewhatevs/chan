import { EditorState } from "@codemirror/state";
import { afterEach, describe, expect, test, vi } from "vitest";

import {
  externalUrlAtPos,
  isOpenableExternalUrl,
  openExternalUrl,
} from "./external_links";
import { chanMarkdown } from "./markdown/grammar";
import { setNotifyHandler } from "../state/notify.svelte";

function state(doc: string): EditorState {
  return EditorState.create({ doc, extensions: [chanMarkdown()] });
}

describe("external link helpers", () => {
  test("recognizes only browser-openable external schemes", () => {
    expect(isOpenableExternalUrl("https://example.com")).toBe(true);
    expect(isOpenableExternalUrl("http://example.com")).toBe(true);
    expect(isOpenableExternalUrl("mailto:a@example.com")).toBe(true);
    expect(isOpenableExternalUrl("tel:+15551212")).toBe(true);
    expect(isOpenableExternalUrl("docs/readme.md")).toBe(false);
    expect(isOpenableExternalUrl("#local")).toBe(false);
    expect(isOpenableExternalUrl("javascript:alert(1)")).toBe(false);
  });

  test("finds markdown link and naked URL targets at document positions", () => {
    const linkDoc = "[Example](https://example.com) and https://chan.app";
    const linkState = state(linkDoc);

    expect(externalUrlAtPos(linkState, linkDoc.indexOf("Example") + 2)).toBe(
      "https://example.com",
    );
    expect(externalUrlAtPos(linkState, linkDoc.indexOf("chan.app"))).toBe(
      "https://chan.app",
    );
    expect(externalUrlAtPos(state("[Local](notes.md)"), 2)).toBeNull();
  });

  test("uses the Tauri opener bridge when available", async () => {
    const openUrl = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: { opener: { openUrl } },
    });

    await expect(openExternalUrl("https://example.com")).resolves.toBe(true);
    expect(openUrl).toHaveBeenCalledWith("https://example.com");

    delete (window as unknown as { __TAURI__?: unknown }).__TAURI__;
  });

  test("falls back to the Tauri invoke bridge when opener is unavailable", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: { invoke },
    });

    await expect(openExternalUrl("https://example.com")).resolves.toBe(true);
    expect(invoke).toHaveBeenCalledWith("plugin:opener|open_url", {
      url: "https://example.com",
    });

    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  test("uses window.open outside Tauri", async () => {
    const open = vi.spyOn(window, "open").mockReturnValue(null);

    await expect(openExternalUrl("https://example.com")).resolves.toBe(true);
    expect(open).toHaveBeenCalledWith(
      "https://example.com",
      "_blank",
      "noopener,noreferrer",
    );
  });
});

describe("openExternalUrl no-default-browser fallback", () => {
  afterEach(() => {
    delete (window as unknown as { __TAURI__?: unknown }).__TAURI__;
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    setNotifyHandler((msg) => console.warn(msg));
  });

  test("surfaces a status message and copies URL when the Tauri opener throws", async () => {
    const openUrl = vi.fn().mockRejectedValue(new Error("no app found"));
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: { opener: { openUrl } },
    });
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const notifications: string[] = [];
    setNotifyHandler((msg) => notifications.push(msg));

    await expect(openExternalUrl("https://example.com")).resolves.toBe(false);

    expect(openUrl).toHaveBeenCalledWith("https://example.com");
    expect(writeText).toHaveBeenCalledWith("https://example.com");
    expect(notifications).toEqual([
      "Couldn't open link in browser - URL copied to clipboard",
    ]);
  });

  test("falls back to including the raw URL when clipboard also fails", async () => {
    const openUrl = vi.fn().mockRejectedValue(new Error("opener denied"));
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: { opener: { openUrl } },
    });
    const writeText = vi.fn().mockRejectedValue(new Error("clipboard denied"));
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const notifications: string[] = [];
    setNotifyHandler((msg) => notifications.push(msg));

    await expect(openExternalUrl("https://example.com")).resolves.toBe(false);

    expect(notifications).toEqual([
      "Couldn't open link in browser - https://example.com",
    ]);
  });

  test("does not fall back to window.open inside the Tauri webview", async () => {
    const openUrl = vi.fn().mockRejectedValue(new Error("no app found"));
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: { opener: { openUrl } },
    });
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
    setNotifyHandler(() => {});
    const open = vi.spyOn(window, "open").mockReturnValue(null);
    open.mockClear();

    await openExternalUrl("https://example.com");

    expect(open).not.toHaveBeenCalled();
  });
});
