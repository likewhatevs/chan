<script lang="ts">
  import { tick } from "svelte";
  import { Clipboard, RotateCcw, Search } from "lucide-svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { SearchAddon } from "@xterm/addon-search";
  import { SerializeAddon } from "@xterm/addon-serialize";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import "@xterm/xterm/css/xterm.css";
  import { withTokenQuery } from "../api/client";
  import type { TerminalTab as TerminalTabState } from "../state/tabs.svelte";

  let {
    tab,
    active,
  }: {
    tab: TerminalTabState;
    active: boolean;
  } = $props();

  type ServerFrame =
    | { type: "ready"; cols: number; rows: number }
    | { type: "exit"; code: number }
    | { type: "error"; message: string };

  let host: HTMLDivElement | undefined = $state();
  let searchInput: HTMLInputElement | undefined = $state();
  let term: Terminal | null = null;
  let fit: FitAddon | null = null;
  let search: SearchAddon | null = null;
  let serialize: SerializeAddon | null = null;
  let ws: WebSocket | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let status = $state<"closed" | "connecting" | "connected" | "exited">("closed");
  let statusDetail = $state("");
  let findOpen = $state(false);
  let findQuery = $state("");

  $effect(() => {
    if (!host || term) return;
    void tick().then(start);
    return teardown;
  });

  $effect(() => {
    if (!active) return;
    queueFit();
    queueMicrotask(() => term?.focus());
  });

  function start(): void {
    if (!host || term) return;
    const styles = getComputedStyle(document.documentElement);
    const bg = styles.getPropertyValue("--bg").trim() || "#1c1c1e";
    const text = styles.getPropertyValue("--text").trim() || "#ebebf0";
    const cursor = styles.getPropertyValue("--link").trim() || "#58a6ff";
    term = new Terminal({
      allowTransparency: false,
      cursorBlink: true,
      cursorStyle: "block",
      fontFamily:
        'SFMono-Regular, ui-monospace, Menlo, Consolas, "Liberation Mono", monospace',
      fontSize: 13,
      lineHeight: 1.15,
      scrollback: 20_000,
      tabStopWidth: 8,
      theme: {
        background: bg,
        foreground: text,
        cursor,
        selectionBackground: "rgba(88, 166, 255, 0.35)",
        black: "#0c0c0d",
        red: "#ff6b6b",
        green: "#6cd07a",
        yellow: "#e3b341",
        blue: "#58a6ff",
        magenta: "#b07dff",
        cyan: "#5dd8d8",
        white: "#d8d8de",
        brightBlack: "#6c6c70",
        brightRed: "#ff8585",
        brightGreen: "#8be89a",
        brightYellow: "#f2d16b",
        brightBlue: "#7dbdff",
        brightMagenta: "#c8a6ff",
        brightCyan: "#7df0f0",
        brightWhite: "#ffffff",
      },
    });
    fit = new FitAddon();
    search = new SearchAddon({ highlightLimit: 1000 });
    serialize = new SerializeAddon();
    term.loadAddon(fit);
    term.loadAddon(search);
    term.loadAddon(serialize);
    term.loadAddon(new WebLinksAddon());
    term.open(host);
    term.onData((data) => send({ type: "input", data }));
    term.onResize(({ cols, rows }) => send({ type: "resize", cols, rows }));
    resizeObserver = new ResizeObserver(queueFit);
    resizeObserver.observe(host);
    queueFit();
    connect();
    if (active) queueMicrotask(() => term?.focus());
  }

  function connect(): void {
    if (!term) return;
    closeSocket();
    status = "connecting";
    statusDetail = "";
    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    const path = withTokenQuery(
      `/api/terminal/ws?cols=${term.cols}&rows=${term.rows}`,
    );
    ws = new WebSocket(`${proto}//${window.location.host}${path}`);
    ws.binaryType = "arraybuffer";
    ws.onopen = () => {
      status = "connected";
      statusDetail = `${term?.cols ?? 0}x${term?.rows ?? 0}`;
      if (term) send({ type: "resize", cols: term.cols, rows: term.rows });
    };
    ws.onmessage = async (event) => {
      if (event.data instanceof ArrayBuffer) {
        term?.write(new Uint8Array(event.data));
        return;
      }
      if (event.data instanceof Blob) {
        term?.write(new Uint8Array(await event.data.arrayBuffer()));
        return;
      }
      let frame: ServerFrame;
      try {
        frame = JSON.parse(String(event.data)) as ServerFrame;
      } catch {
        return;
      }
      if (frame.type === "ready") {
        statusDetail = `${frame.cols}x${frame.rows}`;
      } else if (frame.type === "exit") {
        status = "exited";
        statusDetail = `exit ${frame.code}`;
      } else if (frame.type === "error") {
        statusDetail = frame.message;
        term?.writeln(`\r\nterminal error: ${frame.message}`);
      }
    };
    ws.onclose = () => {
      if (status !== "exited") status = "closed";
    };
    ws.onerror = () => {
      statusDetail = "connection failed";
    };
  }

  function send(frame: unknown): void {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify(frame));
  }

  function queueFit(): void {
    requestAnimationFrame(() => {
      try {
        fit?.fit();
        if (term) statusDetail = `${term.cols}x${term.rows}`;
      } catch {
        // xterm throws if fit runs before dimensions settle.
      }
    });
  }

  function closeSocket(): void {
    const s = ws;
    ws = null;
    if (!s) return;
    s.onopen = null;
    s.onclose = null;
    s.onerror = null;
    s.onmessage = null;
    try {
      s.close();
    } catch {
      // Already closed.
    }
  }

  function teardown(): void {
    closeSocket();
    resizeObserver?.disconnect();
    resizeObserver = null;
    term?.dispose();
    term = null;
    fit = null;
    search = null;
    serialize = null;
  }

  function restart(): void {
    teardown();
    void tick().then(start);
  }

  async function copyScrollback(): Promise<void> {
    const text = serialize?.serialize({ scrollback: 20_000 }) ?? "";
    if (!text) return;
    await navigator.clipboard?.writeText(text);
    term?.focus();
  }

  function openFind(): void {
    findOpen = true;
    void tick().then(() => searchInput?.focus());
  }

  function runFind(next: boolean): void {
    if (!findQuery.trim()) {
      search?.clearDecorations();
      return;
    }
    const opts = {
      decorations: {
        matchBackground: "#7c5cff",
        matchOverviewRuler: "#7c5cff",
        activeMatchBackground: "#58a6ff",
        activeMatchColorOverviewRuler: "#58a6ff",
      },
    };
    if (next) search?.findNext(findQuery, opts);
    else search?.findPrevious(findQuery, opts);
  }

  function onFindKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      findOpen = false;
      search?.clearDecorations();
      term?.focus();
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      runFind(!e.shiftKey);
    }
  }

  function onShellKeydown(e: KeyboardEvent): void {
    if (
      (e.metaKey || e.ctrlKey) &&
      !e.shiftKey &&
      !e.altKey &&
      e.key.toLowerCase() === "f"
    ) {
      e.preventDefault();
      openFind();
    }
  }
</script>

<div
  class="terminal-tab"
  class:active
  data-terminal-tab-id={tab.id}
  role="tabpanel"
  aria-hidden={!active}
  onkeydown={onShellKeydown}
>
  <header>
    <span class:connected={status === "connected"} class="status">
      {status}{statusDetail ? ` - ${statusDetail}` : ""}
    </span>
    {#if findOpen}
      <input
        bind:this={searchInput}
        class="find"
        value={findQuery}
        placeholder="find"
        spellcheck="false"
        oninput={(e) => {
          findQuery = (e.currentTarget as HTMLInputElement).value;
          runFind(true);
        }}
        onkeydown={onFindKeydown}
      />
    {/if}
    <button type="button" class="tool-btn" onclick={openFind} title="Find" aria-label="Find">
      <Search size={14} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <button
      type="button"
      class="tool-btn"
      onclick={copyScrollback}
      title="Copy scrollback"
      aria-label="Copy scrollback"
    >
      <Clipboard size={14} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <button type="button" class="tool-btn" onclick={restart} title="Restart" aria-label="Restart">
      <RotateCcw size={14} strokeWidth={1.75} aria-hidden="true" />
    </button>
  </header>
  <div class="terminal-host" bind:this={host}></div>
</div>

<style>
  .terminal-tab {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--bg);
    color: var(--text);
    visibility: hidden;
    pointer-events: none;
  }
  .terminal-tab.active {
    visibility: visible;
    pointer-events: auto;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-card);
    flex-shrink: 0;
  }
  .status {
    color: var(--text-secondary);
    font-size: 12px;
    margin-right: auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .status.connected {
    color: var(--accent);
  }
  .find {
    width: min(220px, 28vw);
    min-width: 96px;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 7px;
    font: inherit;
    font-size: 13px;
    outline: none;
  }
  .find:focus {
    border-color: var(--link);
  }
  .tool-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 24px;
    padding: 0;
    background: var(--bg);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    flex-shrink: 0;
  }
  .tool-btn:hover {
    color: var(--text);
    border-color: var(--btn-hover);
  }
  .terminal-host {
    flex: 1;
    min-height: 0;
    padding: 8px;
    background: var(--bg);
    overflow: hidden;
  }
  .terminal-host :global(.xterm) {
    height: 100%;
  }
  .terminal-host :global(.xterm-viewport) {
    scrollbar-color: var(--separator) var(--bg);
  }
  @media (max-width: 640px) {
    header {
      gap: 6px;
      padding: 6px;
    }
    .find {
      width: 112px;
    }
  }
</style>
