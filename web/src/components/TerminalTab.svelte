<script lang="ts">
  import { tick } from "svelte";
  import { Check, Clipboard, Pencil, Radio, RotateCcw, Search } from "lucide-svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { SearchAddon } from "@xterm/addon-search";
  import { SerializeAddon } from "@xterm/addon-serialize";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import "@xterm/xterm/css/xterm.css";
  import { withTokenQuery } from "../api/client";
  import { chordFor } from "../state/shortcuts";
  import {
    allTerminalTabs,
    broadcastTerminalInput,
    registerTerminalInputSink,
    renameTerminalTab,
    setTerminalBroadcastEnabled,
    setTerminalBroadcastTarget,
    terminalTabName,
    type TerminalTab as TerminalTabState,
  } from "../state/tabs.svelte";
  import { clampMenu } from "./menuClamp";
  import {
    closeTabMenu,
    tabMenu,
  } from "../state/tabMenu.svelte";

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
  const menuOpen = $derived(tabMenu.openForTabId === tab.id);
  const menuPos = $derived.by(() => {
    const a = tabMenu.anchor;
    if (!a) return { x: 0, y: 0 };
    return { x: Math.round(a.left), y: Math.round(a.bottom + 4) };
  });
  const otherTerminalTabs = $derived(
    allTerminalTabs().filter((candidate) => candidate.id !== tab.id),
  );
  const selectedBroadcastTargets = $derived(new Set(tab.broadcastTargetIds));
  const broadcastChord = chordFor("app.terminal.broadcast.toggle") ?? "";

  $effect(() => {
    if (!host || term) return;
    void tick().then(start);
    return teardown;
  });

  $effect(() => registerTerminalInputSink(tab.id, (data) => sendInput(data)));

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
    term.onData((data) => {
      sendInput(data);
      broadcastTerminalInput(tab, data);
    });
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
    const params = new URLSearchParams({
      cols: String(term.cols),
      rows: String(term.rows),
      tab_name: terminalTabName(tab),
    });
    const path = withTokenQuery(`/api/terminal/ws?${params.toString()}`);
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

  function sendInput(data: string): void {
    send({ type: "input", data });
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

  function onMenuKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape" && menuOpen) {
      e.preventDefault();
      closeTabMenu();
    }
  }

  function onDocPointerDown(e: PointerEvent): void {
    if (!menuOpen) return;
    const t = e.target as Element | null;
    if (!t) return;
    if (t.closest(".terminal-tab-menu-bubble")) return;
    if (t.closest(".tab")) return;
    closeTabMenu();
  }

  function toggleBroadcast(): void {
    setTerminalBroadcastEnabled(tab, !tab.broadcastEnabled);
  }
</script>

<svelte:window onkeydown={onMenuKeydown} onpointerdown={onDocPointerDown} />

<div
  class="terminal-tab"
  class:active
  data-terminal-tab-id={tab.id}
  role="tabpanel"
  aria-hidden={!active}
  onkeydown={onShellKeydown}
>
  {#if menuOpen}
    <div
      class="terminal-tab-menu-bubble"
      role="menu"
      tabindex="-1"
      aria-label="terminal tab menu"
      use:clampMenu={menuPos}
      onmousedown={(e) => e.stopPropagation()}
    >
      <label class="rename-row">
        <span class="rename-label">
          <Pencil size={15} strokeWidth={1.75} aria-hidden="true" />
          <span>Name</span>
        </span>
        <input
          class="rename-input"
          value={tab.title}
          spellcheck="false"
          oninput={(e) => renameTerminalTab(tab, (e.currentTarget as HTMLInputElement).value)}
          onkeydown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              closeTabMenu();
              term?.focus();
            }
          }}
        />
      </label>
      <div class="action-list">
        <button class="mbtn" class:on={tab.broadcastEnabled} onclick={toggleBroadcast}>
          <span class="mbtn-icon">
            <Radio size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">
            {tab.broadcastEnabled ? "Broadcast Input On" : "Broadcast Input Off"}
          </span>
          <span class="mbtn-chord">{broadcastChord}</span>
        </button>
        <div class="msep" role="separator"></div>
        {#if otherTerminalTabs.length === 0}
          <div class="empty-targets">No other terminal tabs</div>
        {:else}
          {#each otherTerminalTabs as target (target.id)}
            <label class="target-row">
              <span class="target-check">
                <input
                  type="checkbox"
                  checked={selectedBroadcastTargets.has(target.id)}
                  onchange={(e) =>
                    setTerminalBroadcastTarget(
                      tab,
                      target.id,
                      (e.currentTarget as HTMLInputElement).checked,
                    )}
                />
                {#if selectedBroadcastTargets.has(target.id)}
                  <Check size={13} strokeWidth={2} aria-hidden="true" />
                {/if}
              </span>
              <span class="target-name">{terminalTabName(target)}</span>
            </label>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
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
  .terminal-tab-menu-bubble {
    position: fixed;
    z-index: 50;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.18);
    padding: 6px;
    min-width: 260px;
    max-width: calc(100vw - 16px);
    max-height: calc(100vh - 24px);
    overflow-y: auto;
    color: var(--text);
    font-size: 13px;
    transform-origin: top left;
    animation: bubble-pop 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  @keyframes bubble-pop {
    0% { opacity: 0; transform: scale(0.92); }
    100% { opacity: 1; transform: scale(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .terminal-tab-menu-bubble { animation: none; }
  }
  .rename-row {
    display: grid;
    grid-template-columns: auto minmax(120px, 1fr);
    align-items: center;
    gap: 10px;
    padding: 6px 4px 8px;
    border-bottom: 1px solid var(--separator);
  }
  .rename-label {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    color: var(--text-secondary);
    min-width: 0;
  }
  .rename-input {
    min-width: 0;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 5px 7px;
    font: inherit;
    outline: none;
  }
  .rename-input:focus {
    border-color: var(--link);
  }
  .action-list {
    display: flex;
    flex-direction: column;
    padding-top: 4px;
  }
  .mbtn {
    display: flex;
    align-items: center;
    gap: 8px;
    background: none;
    border: 0;
    border-radius: 4px;
    cursor: pointer;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    padding: 6px 8px;
    text-align: left;
  }
  .mbtn:hover,
  .mbtn.on {
    background: var(--hover-bg);
  }
  .mbtn-icon {
    width: 18px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .mbtn-label,
  .target-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mbtn-chord {
    margin-left: 1.5rem;
    color: var(--text-secondary);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 11.5px;
  }
  .msep {
    height: 1px;
    background: var(--separator, var(--border));
    margin: 4px 2px;
  }
  .empty-targets {
    padding: 7px 8px;
    color: var(--text-secondary);
  }
  .target-row {
    display: flex;
    align-items: center;
    gap: 8px;
    border-radius: 4px;
    padding: 6px 8px;
    cursor: pointer;
  }
  .target-row:hover {
    background: var(--hover-bg);
  }
  .target-check {
    position: relative;
    width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .target-check input {
    position: absolute;
    inset: 0;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }
  .target-check {
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    background: var(--bg);
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
