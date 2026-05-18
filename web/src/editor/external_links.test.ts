import { EditorState } from "@codemirror/state";
import { describe, expect, test, vi } from "vitest";

import {
  externalUrlAtPos,
  isOpenableExternalUrl,
  openExternalUrl,
} from "./external_links";
import { chanMarkdown } from "./markdown/grammar";

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
