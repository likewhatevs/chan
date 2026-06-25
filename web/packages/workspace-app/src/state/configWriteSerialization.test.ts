// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import storeSource from "./store.svelte.ts?raw";
import configWriteSource from "./configWrite.ts?raw";
import editorToolsSource from "./editorTools.svelte.ts?raw";
import { api } from "../api/client";
import { updateGlobalConfigSerial } from "./store.svelte";

// PATCH /api/config is a whole-block replacement. Independent read-modify-write
// chains (the old per-persister inflight model) can interleave: a terminal-config
// autosave reads the config before a just-fired hybrid_surface_themes override
// PATCH lands, then writes the block back without the override — so the override
// resets on reload. updateGlobalConfigSerial funnels every write through one
// chain so this can't happen.

type Cfg = {
  preferences: Record<string, unknown>;
  workspaces: unknown[];
};

let server: Cfg;

function jsonResponse(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}

beforeEach(() => {
  server = {
    preferences: { theme: "dark", terminal: { default_term: "xterm-256color" } },
    workspaces: [],
  };
  vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
    const url = typeof input === "string" ? input : input.toString();
    const method = init?.method ?? "GET";
    if (url.includes("/api/config")) {
      if (method === "PATCH") {
        server = JSON.parse(String(init?.body)) as Cfg;
        return jsonResponse(server);
      }
      return jsonResponse(server);
    }
    return new Response(null, { status: 404 });
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("config write race (the latent hybrid-theme reset bug)", () => {
  test("independent stale read-modify-write clobbers a field the other writer owns", async () => {
    // Two writers both read the current config (no override, default
    // terminal), then both whole-block PATCH. The terminal write, built
    // from the stale read, drops the override the theme write saved.
    const themeRead = await api.config();
    const terminalRead = await api.config();
    await api.updateConfig({
      ...themeRead,
      preferences: {
        ...themeRead.preferences,
        hybrid_surface_themes: { terminal: "light" },
      },
    });
    await api.updateConfig({
      ...terminalRead,
      preferences: {
        ...terminalRead.preferences,
        terminal: {
          ...terminalRead.preferences.terminal,
          default_term: "tmux-256color",
        },
      },
    });
    // The override is gone — this is the reset users see on reload.
    expect(server.preferences.hybrid_surface_themes).toBeUndefined();
  });
});

describe("updateGlobalConfigSerial (the fix)", () => {
  test("concurrent writes to different fields both survive", async () => {
    await Promise.all([
      updateGlobalConfigSerial((prefs) => ({
        ...prefs,
        hybrid_surface_themes: { terminal: "light" },
      })),
      updateGlobalConfigSerial((prefs) => ({
        ...prefs,
        terminal: { ...prefs.terminal, default_term: "tmux-256color" },
      })),
    ]);
    expect(server.preferences.hybrid_surface_themes).toEqual({
      terminal: "light",
    });
    expect(
      (server.preferences.terminal as { default_term: string }).default_term,
    ).toBe("tmux-256color");
  });

  test("a mutation returning null skips the PATCH", async () => {
    const patches = () =>
      (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls.filter(
        ([, init]) => (init as RequestInit | undefined)?.method === "PATCH",
      ).length;
    const before = patches();
    await updateGlobalConfigSerial(() => null);
    expect(patches()).toBe(before);
  });
});

describe("config writers route through the shared serial chain (source pins)", () => {
  test("persistHybridSurfaceThemes uses updateGlobalConfigSerial", () => {
    expect(storeSource).toMatch(
      /function persistHybridSurfaceThemes\(\): Promise<void> \{[\s\S]*?return updateGlobalConfigSerial\(\(prefs\) => \(\{[\s\S]*?hybrid_surface_themes: next,/,
    );
  });

  test("persistThemeChoice + persistPaneWidths route through updateGlobalConfigSerial", () => {
    expect(storeSource).toMatch(
      /function persistThemeChoice\([\s\S]*?return updateGlobalConfigSerial\(/,
    );
    expect(storeSource).toMatch(
      /persistPaneWidths\(\): void \{[\s\S]*?updateGlobalConfigSerial\(\(prefs\) => \{/,
    );
  });

  test("no per-persister inflight chains remain for the config writers", () => {
    expect(storeSource).not.toMatch(/hybridSurfaceThemePersistInflight/);
    expect(storeSource).not.toMatch(/themePersistInflight/);
    expect(storeSource).not.toMatch(/widthsPersistInflight/);
    expect(editorToolsSource).not.toMatch(/stripWhitespacePersistInflight/);
  });

  test("the serializer is a leaf module re-exported by store (no import cycle)", () => {
    // store imports editorTools, so the shared chain must live in a module
    // that neither imports back. configWrite depends only on the api client.
    expect(configWriteSource).toMatch(
      /export function updateGlobalConfigSerial\(/,
    );
    expect(configWriteSource).not.toMatch(/from "\.\/store\.svelte"/);
    expect(storeSource).toMatch(
      /import \{ updateGlobalConfigSerial \} from "\.\/configWrite";/,
    );
    expect(storeSource).toMatch(/export \{ updateGlobalConfigSerial \};/);
  });

  test("editorTools persists through the shared chain, imported from configWrite", () => {
    expect(editorToolsSource).toMatch(
      /import \{ updateGlobalConfigSerial \} from "\.\/configWrite";/,
    );
    expect(editorToolsSource).toMatch(
      /persistStripTrailingWhitespaceOnSave\(value: boolean\): Promise<void> \{[\s\S]*?return updateGlobalConfigSerial\(/,
    );
  });
});
