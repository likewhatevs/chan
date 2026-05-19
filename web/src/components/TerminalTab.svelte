<script lang="ts">
  import { tick } from "svelte";
  import {
    Check,
    Clipboard,
    ClipboardPaste,
    FilePlus,
    FolderOpen,
    History,
    Info,
    MessageSquareText,
    Network,
    Pencil,
    Radio,
    RotateCcw,
    Search,
    Settings,
    SquareSplitHorizontal,
    SquareSplitVertical,
    Terminal as TerminalIcon,
  } from "lucide-svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { SearchAddon } from "@xterm/addon-search";
  import { SerializeAddon } from "@xterm/addon-serialize";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import "@xterm/xterm/css/xterm.css";
  import { api, sessionWindowId, withTokenQuery } from "../api/client";
  import type { TerminalSpawnResponse } from "../api/types";
  import { chordFor } from "../state/shortcuts";
  import {
    advanceTerminalSeq,
    allTerminalTabs,
    broadcastTerminalInput,
    canReopenClosedTab,
    canSplit,
    closeTab,
    clearTerminalSession,
    dismissTerminalEnvNamePrompt,
    layout,
    openTerminalInActivePane,
    openTerminalInPane,
    registerTerminalCloseSink,
    registerTerminalInputSink,
    renameTerminalTab,
    reopenClosedTab,
    setTerminalBroadcastEnabled,
    setTerminalBroadcastTarget,
    setTerminalActivity,
    setTerminalMcpEnv,
    setTerminalSession,
    splitActive,
    terminalBroadcastMemberIds,
    terminalEnvTabNameStale,
    terminalMcpEnvEnabled,
    terminalTabName,
    type TerminalTab as TerminalTabState,
  } from "../state/tabs.svelte";
  import {
    drive,
    fileOps,
    openFsGraphForDirectory,
    openSettings,
    revealPathInBrowser,
    scheduleSessionSave,
    searchPanel,
    ui,
  } from "../state/store.svelte";
  import { terminalWsPath } from "../terminal/session";
  import { handleTerminalMetaKey } from "../terminal/keymap";
  import { injectShowMcpEnvCommand } from "../terminal/mcpEnv";
  import { uiConfirm } from "../state/confirm.svelte";
  import { clampMenu } from "./menuClamp";
  import {
    closeTabMenu,
    openTabMenu,
    tabMenu,
  } from "../state/tabMenu.svelte";
  import BubbleOverlay from "./BubbleOverlay.svelte";
  import TerminalRichPrompt from "./TerminalRichPrompt.svelte";
  import { readWatcherEvents } from "../state/watcherEvents";

  let {
    tab,
    paneId,
    active,
    focused,
  }: {
    tab: TerminalTabState;
    paneId: string;
    active: boolean;
    focused: boolean;
  } = $props();

  type ServerFrame =
    | { type: "ready"; cols: number; rows: number; cwd?: string | null }
    | {
        type: "session";
        id: string;
        seq: number;
        missed_bytes?: number;
        bytes_since_focus?: number;
      }
    | { type: "activity"; bytes_since_focus: number }
    | { type: "cwd"; cwd?: string | null }
    | { type: "resize_other"; cols: number; rows: number }
    | { type: "closed"; reason: CloseReason }
    | { type: "exit"; code: number }
    | { type: "error"; message?: string; reason?: string };

  type CloseReason = "idle" | "drive" | "shutdown" | "explicit" | "capped" | "error";

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
  let missedBytes = $state(0);
  let sessionClosedReason = $state<CloseReason | null>(null);
  let findOpen = $state(false);
  let findQuery = $state("");
  let mcpInfoOpen = $state(false);
  let sawSessionControl = false;
  let pendingPromptSeed = "";
  let promptSeedSent = false;
  let terminalCwdAbs: string | null = $state(null);
  let watcherPollTimer: ReturnType<typeof setInterval> | null = null;
  const outputDecoder = new TextDecoder();
  let lastSessionSave = 0;
  let sessionSaveTimer: ReturnType<typeof setTimeout> | null = null;
  const menuOpen = $derived(tabMenu.openForTabId === tab.id);
  const menuPos = $derived.by(() => {
    const a = tabMenu.anchor;
    if (!a) return { x: 0, y: 0 };
    return { x: Math.round(a.left), y: Math.round(a.bottom + 4) };
  });
  const broadcastTargets = $derived(allTerminalTabs().filter((candidate) => candidate.id !== tab.id));
  const selectedBroadcastTargets = $derived(new Set(terminalBroadcastMemberIds(tab)));
  const allBroadcastTargetsSelected = $derived(
    broadcastTargets.length > 0 &&
      broadcastTargets.every((target) => selectedBroadcastTargets.has(target.id)),
  );
  const mcpEnvOn = $derived(terminalMcpEnvEnabled(tab));
  const watcherPath = $derived(tab.watcher?.path ?? null);
  const showMcpEnvDisabled = $derived(tab.sessionMcpEnv === false);
  const staleEnvName = $derived(terminalEnvTabNameStale(tab));
  const showStaleEnvPrompt = $derived(
    staleEnvName && !tab.terminalEnvNamePromptDismissed,
  );
  const splitsAllowed = $derived.by(() => {
    void layout.rootId;
    void Object.keys(layout.nodes).length;
    return canSplit();
  });

  $effect(() => {
    if (!host || term) return;
    void tick().then(start);
    return teardown;
  });

  $effect(() => {
    const unregisterInput = registerTerminalInputSink(tab.id, (data) => sendInput(data));
    const unregisterClose = registerTerminalCloseSink(tab.id, explicitCloseSession);
    return () => {
      unregisterInput();
      unregisterClose();
    };
  });

  $effect(() => {
    if (!focused) return;
    queueFit();
    setTerminalActivity(tab, false);
    sendFocusState();
    queueMicrotask(() => term?.focus());
  });

  $effect(() => {
    if (focused) return;
    sendFocusState();
  });

  $effect(() => {
    if (!watcherPath) {
      if (watcherPollTimer) clearInterval(watcherPollTimer);
      watcherPollTimer = null;
      return;
    }
    void refreshWatcherEvents();
    watcherPollTimer = setInterval(() => void refreshWatcherEvents(), 5000);
    return () => {
      if (watcherPollTimer) clearInterval(watcherPollTimer);
      watcherPollTimer = null;
    };
  });

  $effect(() => {
    ui.theme;
    applyTerminalTheme();
  });

  function terminalTheme() {
    const styles = getComputedStyle(document.documentElement);
    const bg = styles.getPropertyValue("--bg").trim() || "#1c1c1e";
    const text = styles.getPropertyValue("--text").trim() || "#ebebf0";
    const cursor = styles.getPropertyValue("--link").trim() || "#58a6ff";
    const base = {
      background: bg,
      foreground: text,
      cursor,
      selectionBackground: "rgba(88, 166, 255, 0.35)",
    };
    if (ui.theme === "light") {
      return {
        ...base,
        black: "#24292f",
        red: "#cf222e",
        green: "#1a7f37",
        yellow: "#8a6300",
        blue: "#0969da",
        magenta: "#8250df",
        cyan: "#1b7c83",
        white: "#4b5563",
        brightBlack: "#57606a",
        brightRed: "#a40e26",
        brightGreen: "#116329",
        brightYellow: "#6f4e00",
        brightBlue: "#0550ae",
        brightMagenta: "#6639ba",
        brightCyan: "#0a6b73",
        brightWhite: "#6e7781",
      };
    }
    return {
      ...base,
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
    };
  }

  function applyTerminalTheme(): void {
    if (!term) return;
    term.options.theme = terminalTheme();
  }

  function start(): void {
    if (!host || term) return;
    term = new Terminal({
      allowTransparency: false,
      cursorBlink: true,
      cursorStyle: "block",
      fontFamily:
        'SFMono-Regular, ui-monospace, Menlo, Consolas, "Liberation Mono", monospace',
      fontSize: 13,
      lineHeight: 1.15,
      macOptionIsMeta: true,
      scrollback: 20_000,
      tabStopWidth: 8,
      theme: terminalTheme(),
    });
    fit = new FitAddon();
    search = new SearchAddon({ highlightLimit: 1000 });
    serialize = new SerializeAddon();
    term.loadAddon(fit);
    term.loadAddon(search);
    term.loadAddon(serialize);
    term.loadAddon(new WebLinksAddon());
    term.open(host);
    term.attachCustomKeyEventHandler(handleTerminalKeyEvent);
    term.onData(sendUserInput);
    term.onResize(({ cols, rows }) => send({ type: "resize", cols, rows }));
    resizeObserver = new ResizeObserver(queueFit);
    resizeObserver.observe(host);
    queueFit();
    connect();
    if (focused) queueMicrotask(() => term?.focus());
  }

  function connect(): void {
    if (!term) return;
    closeSocket();
    status = "connecting";
    statusDetail = "";
    missedBytes = 0;
    sessionClosedReason = null;
    sawSessionControl = false;
    const reattaching = Boolean(tab.terminalSessionId);
    pendingPromptSeed = reattaching ? "" : (tab.seedInput ?? "");
    promptSeedSent = false;
    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    const path = withTokenQuery(
      terminalWsPath({
        cols: term.cols,
        rows: term.rows,
        tabName: terminalTabName(tab),
        windowId: sessionWindowId(),
        sessionId: tab.terminalSessionId,
        lastSeq: tab.lastSeq,
        mcpEnv: mcpEnvOn,
        cwd: reattaching ? undefined : tab.cwd,
      }),
    );
    ws = new WebSocket(`${proto}//${window.location.host}${path}`);
    ws.binaryType = "arraybuffer";
    ws.onopen = () => {
      status = "connected";
      statusDetail = `${term?.cols ?? 0}x${term?.rows ?? 0}`;
      if (term) send({ type: "resize", cols: term.cols, rows: term.rows });
      sendFocusState();
    };
    ws.onmessage = async (event) => {
      if (event.data instanceof ArrayBuffer) {
        const bytes = new Uint8Array(event.data);
        term?.write(bytes);
        recordOutputBytes(bytes.byteLength);
        maybeRefreshWatcher(bytes);
        maybeSeedPrompt();
        return;
      }
      if (event.data instanceof Blob) {
        const bytes = new Uint8Array(await event.data.arrayBuffer());
        term?.write(bytes);
        recordOutputBytes(bytes.byteLength);
        maybeRefreshWatcher(bytes);
        maybeSeedPrompt();
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
        terminalCwdAbs = frame.cwd ?? null;
      } else if (frame.type === "session") {
        sawSessionControl = true;
        setTerminalSession(tab, frame.id, frame.seq, mcpEnvOn);
        setTerminalActivity(tab, !focused && (frame.bytes_since_focus ?? 0) > 0);
        scheduleTerminalSessionSave();
        missedBytes = Math.max(0, Math.floor(frame.missed_bytes ?? 0));
        status = "connected";
        statusDetail = `session ${frame.id.slice(0, 8)}`;
        if (missedBytes > 0) {
          term?.writeln(`\r\nterminal replay missed ${missedBytes} bytes`);
        }
      } else if (frame.type === "resize_other") {
        term?.resize(frame.cols, frame.rows);
        statusDetail = `${frame.cols}x${frame.rows}`;
      } else if (frame.type === "cwd") {
        terminalCwdAbs = frame.cwd ?? null;
      } else if (frame.type === "activity") {
        setTerminalActivity(tab, !focused && frame.bytes_since_focus > 0);
      } else if (frame.type === "closed") {
        sessionClosedReason = frame.reason;
        status = "exited";
        statusDetail = `session ended (${frame.reason})`;
        clearTerminalSession(tab);
        scheduleTerminalSessionSave();
        term?.writeln(`\r\nsession ended (${frame.reason})`);
      } else if (frame.type === "exit") {
        status = "exited";
        statusDetail = `exit ${frame.code}`;
        clearTerminalSession(tab);
        scheduleTerminalSessionSave();
        term?.writeln(`\r\nprocess exited (${frame.code}); press Ctrl+D to close this tab`);
      } else if (frame.type === "error") {
        const detail = frame.message ?? frame.reason ?? "unknown error";
        statusDetail = detail;
        term?.writeln(`\r\nterminal error: ${detail}`);
      }
    };
    ws.onclose = () => {
      if (tab.terminalSessionId && !sawSessionControl && status === "connecting") {
        clearTerminalSession(tab);
        scheduleTerminalSessionSave();
      }
      if (status !== "exited") status = "closed";
    };
    ws.onerror = () => {
      statusDetail = "connection failed";
      if (tab.terminalSessionId && !sawSessionControl) {
        clearTerminalSession(tab);
        scheduleTerminalSessionSave();
      }
    };
  }

  function recordOutputBytes(bytes: number): void {
    advanceTerminalSeq(tab, bytes);
    scheduleTerminalSessionSave();
  }

  function sendFocusState(): void {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    send({ type: "focus", focused });
  }

  function maybeSeedPrompt(): void {
    if (!pendingPromptSeed || promptSeedSent) return;
    promptSeedSent = true;
    const seed = ` ${pendingPromptSeed}\x01`;
    tab.seedInput = undefined;
    setTimeout(() => {
      sendInput(seed);
      term?.focus();
      scheduleTerminalSessionSave();
    }, 150);
  }

  function scheduleTerminalSessionSave(): void {
    const now = Date.now();
    const elapsed = now - lastSessionSave;
    if (elapsed >= 1000) {
      lastSessionSave = now;
      scheduleSessionSave();
      return;
    }
    if (sessionSaveTimer) return;
    sessionSaveTimer = setTimeout(() => {
      sessionSaveTimer = null;
      lastSessionSave = Date.now();
      scheduleSessionSave();
    }, 1000 - elapsed);
  }

  function send(frame: unknown): void {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify(frame));
  }

  function sendInput(data: string): void {
    send({ type: "input", data });
  }

  function sendUserInput(data: string): void {
    sendInput(data);
    broadcastTerminalInput(tab, data);
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
    if (sessionSaveTimer) {
      clearTimeout(sessionSaveTimer);
      sessionSaveTimer = null;
    }
    closeSocket();
    resizeObserver?.disconnect();
    resizeObserver = null;
    term?.dispose();
    term = null;
    fit = null;
    search = null;
    serialize = null;
  }

  async function restart(): Promise<void> {
    closeTabMenu();
    if (tab.terminalSessionId) {
      const confirmed = await uiConfirm({
        title: "Restart terminal?",
        message: "The current terminal session will be closed and replaced.",
        confirmLabel: "Restart",
        destructive: true,
      });
      if (!confirmed) return;
    }
    if (tab.controlledTerminal && tab.terminalSessionId) {
      try {
        await api.restartTerminal(tab.terminalSessionId);
        status = "connecting";
        statusDetail = "restart requested";
      } catch (err) {
        statusDetail = `restart failed: ${(err as Error).message}`;
      }
      return;
    }
    explicitCloseSession();
    teardown();
    void tick().then(start);
  }

  function explicitCloseSession(): void {
    if (tab.terminalSessionId) {
      send({ type: "close" });
      clearTerminalSession(tab);
      scheduleTerminalSessionSave();
    }
  }

  async function copyScrollback(): Promise<void> {
    closeTabMenu();
    const text = serialize?.serialize({ scrollback: 20_000 }) ?? "";
    if (!text) return;
    await navigator.clipboard?.writeText(text);
    term?.focus();
  }

  async function copySelectionOrScrollback(): Promise<void> {
    closeTabMenu();
    const text = term?.getSelection() || serialize?.serialize({ scrollback: 20_000 }) || "";
    if (!text) return;
    await navigator.clipboard?.writeText(text);
    term?.focus();
  }

  async function pasteClipboard(): Promise<void> {
    closeTabMenu();
    const text = await navigator.clipboard?.readText();
    if (text) sendUserInput(text);
    term?.focus();
  }

  function openFind(): void {
    closeTabMenu();
    findOpen = true;
    void tick().then(() => searchInput?.focus());
  }

  function openSearch(): void {
    closeTabMenu();
    searchPanel.open = true;
  }

  function openNewFile(): void {
    const cwd = terminalCwdRel();
    if (cwd === null) return terminalCwdUnavailable();
    closeTabMenu();
    void fileOps.createFile(cwd);
  }

  function doReopenClosedTab(): void {
    closeTabMenu();
    reopenClosedTab();
  }

  function requestTerminalCwd(): void {
    send({ type: "cwd" });
  }

  function terminalCwdRel(): string | null {
    const abs = terminalCwdAbs;
    const root = drive.info?.root;
    if (!abs || !root) return null;
    const normAbs = abs.replace(/\\/g, "/").replace(/\/+$/, "");
    const normRoot = root.replace(/\\/g, "/").replace(/\/+$/, "");
    if (normAbs === normRoot) return "";
    const prefix = `${normRoot}/`;
    if (!normAbs.startsWith(prefix)) return null;
    return normAbs.slice(prefix.length);
  }

  function terminalCwdUnavailable(): void {
    closeTabMenu();
    requestTerminalCwd();
    ui.status = "PTY did not report CWD";
    term?.focus();
  }

  async function copyTerminalCwd(): Promise<void> {
    const cwd = terminalCwdRel();
    if (cwd === null) return terminalCwdUnavailable();
    closeTabMenu();
    await navigator.clipboard?.writeText(cwd);
    term?.focus();
  }

  function showTerminalCwd(): void {
    const cwd = terminalCwdRel();
    if (cwd === null) return terminalCwdUnavailable();
    closeTabMenu();
    revealPathInBrowser(cwd, { inspectorOpen: true });
    term?.focus();
  }

  function graphTerminalCwd(): void {
    const cwd = terminalCwdRel();
    if (cwd === null) return terminalCwdUnavailable();
    closeTabMenu();
    openFsGraphForDirectory(cwd);
    term?.focus();
  }

  function openSettingsFromMenu(): void {
    closeTabMenu();
    openSettings();
  }

  function openNewTerminal(): void {
    closeTabMenu();
    openTerminalInPane(paneId);
  }

  function ensureRichPrompt(): NonNullable<TerminalTabState["richPrompt"]> {
    if (!tab.richPrompt) {
      tab.richPrompt = {
        buffer: "",
        heightPx: Math.max(220, Math.round((host?.clientHeight ?? 640) / 2)),
        open: false,
        mode: "wysiwyg",
      };
    }
    if (!tab.richPrompt.heightPx) {
      tab.richPrompt.heightPx = Math.max(220, Math.round((host?.clientHeight ?? 640) / 2));
    }
    if (!tab.richPrompt.mode) tab.richPrompt.mode = "wysiwyg";
    return tab.richPrompt;
  }

  function openRichPrompt(): void {
    closeTabMenu();
    ensureRichPrompt().open = true;
    if (tab.watcher) tab.watcher.unread = false;
    scheduleTerminalSessionSave();
    void refreshWatcherEvents();
  }

  function closeRichPrompt(): void {
    ensureRichPrompt().open = false;
    scheduleTerminalSessionSave();
    term?.focus();
  }

  function submitRichPrompt(source: string): void {
    sendUserInput(source);
    scheduleTerminalSessionSave();
    term?.focus();
  }

  function watcherStarted(path: string): void {
    tab.watcher = {
      path,
      events: [],
      seenIds: [],
      unread: false,
      trayExpanded: false,
    };
    scheduleTerminalSessionSave();
    void refreshWatcherEvents();
  }

  function watcherStopped(): void {
    tab.watcher = undefined;
    scheduleTerminalSessionSave();
  }

  function watcherDetached(): void {
    tab.watcher = undefined;
    ui.status = "watcher detached on reload";
    scheduleTerminalSessionSave();
  }

  async function refreshWatcherEvents(): Promise<void> {
    if (!tab.watcher) return;
    tab.watcher.loading = true;
    try {
      const events = await readWatcherEvents(tab.watcher.path);
      const prior = new Set(tab.watcher.seenIds);
      const ids = events.map((event) => event.id);
      const hasNew = ids.some((id) => !prior.has(id));
      tab.watcher.events = events;
      tab.watcher.seenIds = ids;
      tab.watcher.error = undefined;
      if (hasNew && !tab.richPrompt?.open) tab.watcher.unread = true;
    } catch (err) {
      tab.watcher.error = `watch read failed: ${(err as Error).message}`;
    } finally {
      if (tab.watcher) tab.watcher.loading = false;
    }
  }

  function maybeRefreshWatcher(bytes: Uint8Array): void {
    if (!tab.watcher) return;
    const text = outputDecoder.decode(bytes, { stream: true });
    if (/\bpoke\r?\n/.test(text)) void refreshWatcherEvents();
  }

  function splitPane(direction: "row" | "column"): void {
    closeTabMenu();
    layout.activePaneId = paneId;
    splitActive(direction);
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

  function isCloseExitedTabKey(e: KeyboardEvent): boolean {
    return (
      e.type === "keydown" &&
      status === "exited" &&
      e.ctrlKey &&
      !e.metaKey &&
      !e.altKey &&
      e.key.toLowerCase() === "d"
    );
  }

  function closeExitedTabFromKey(e: KeyboardEvent): boolean {
    if (!isCloseExitedTabKey(e)) return false;
    e.preventDefault();
    void closeTab(paneId, tab.id, { force: true });
    return true;
  }

  function handleTerminalKeyEvent(e: KeyboardEvent): boolean {
    if (closeExitedTabFromKey(e)) return false;
    if (
      e.type === "keydown" &&
      e.altKey &&
      !e.ctrlKey &&
      !e.metaKey &&
      !e.shiftKey &&
      e.code === "Space"
    ) {
      e.preventDefault();
      openRichPrompt();
      return false;
    }
    return handleTerminalMetaKey(e, sendUserInput);
  }

  function onShellKeydown(e: KeyboardEvent): void {
    if (closeExitedTabFromKey(e)) {
      return;
    }
    if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey && e.code === "Space") {
      e.preventDefault();
      openRichPrompt();
      return;
    }
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

  function focusTerminalTab(tabId: string): void {
    for (const node of Object.values(layout.nodes)) {
      if (node.kind !== "leaf") continue;
      if (!node.tabs.some((candidate) => candidate.id === tabId)) continue;
      node.activeTabId = tabId;
      layout.activePaneId = node.id;
      closeTabMenu();
      return;
    }
  }

  function focusTerminalSession(sessionId: string | undefined): void {
    if (!sessionId) return;
    const found = allTerminalTabs().find((candidate) => candidate.terminalSessionId === sessionId);
    if (found) focusTerminalTab(found.id);
  }

  function focusTerminalName(name: string | undefined): void {
    const target = name?.trim();
    if (!target) return;
    const found = allTerminalTabs().find((candidate) => terminalTabName(candidate) === target);
    if (found) focusTerminalTab(found.id);
  }

  function spawnCreated(response: TerminalSpawnResponse, name: string): void {
    openTerminalInActivePane({
      title: response.tab_label || name,
      sessionId: response.session,
      controlledTerminal: true,
    });
    scheduleTerminalSessionSave();
  }

  function toggleAllBroadcastTargets(): void {
    const select = !allBroadcastTargetsSelected;
    for (const target of broadcastTargets) {
      setTerminalBroadcastTarget(tab, target.id, select);
    }
  }

  function toggleMcpEnv(): void {
    setTerminalMcpEnv(tab, !mcpEnvOn);
    scheduleTerminalSessionSave();
  }

  function showMcpEnv(): void {
    if (showMcpEnvDisabled) return;
    injectShowMcpEnvCommand(sendUserInput);
    term?.focus();
  }

  function onTerminalContextMenu(e: MouseEvent): void {
    e.preventDefault();
    requestTerminalCwd();
    openTabMenu(tab.id, {
      left: e.clientX,
      top: e.clientY,
      right: e.clientX,
      bottom: e.clientY,
    });
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
  oncontextmenu={onTerminalContextMenu}
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
              (e.currentTarget as HTMLInputElement).blur();
            }
          }}
        />
      </label>
      <div class="terminal-status-row">
        <span class:connected={status === "connected"} class="terminal-status">
          {status}{statusDetail ? ` - ${statusDetail}` : ""}
        </span>
        {#if missedBytes > 0}
          <span class="session-note">missed {missedBytes} bytes</span>
        {/if}
        {#if staleEnvName}
          <span class="session-note">stale env</span>
        {/if}
      </div>
      {#if showStaleEnvPrompt}
        <div class="env-stale-row">
          <span>Tab name changed. $CHAN_TAB_NAME will stay at {tab.terminalEnvTabName} until restart.</span>
          <button type="button" onclick={() => void restart()}>Restart now</button>
          <button type="button" onclick={() => dismissTerminalEnvNamePrompt(tab)}>Later</button>
        </div>
      {/if}
      <div class="action-list">
        {#if sessionClosedReason}
          <button class="mbtn" onclick={() => void restart()}>
            <span class="mbtn-icon">
              <RotateCcw size={16} strokeWidth={1.75} aria-hidden="true" />
            </span>
            <span class="mbtn-label">Start New Session</span>
            <span class="mbtn-chord"></span>
          </button>
        {/if}
        <button class="mbtn" onclick={copySelectionOrScrollback}>
          <span class="mbtn-icon">
            <Clipboard size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Copy</span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={pasteClipboard}>
          <span class="mbtn-icon">
            <ClipboardPaste size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Paste</span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={openRichPrompt}>
          <span class="mbtn-icon">
            <MessageSquareText size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Rich prompt</span>
          <span class="mbtn-chord">Alt+Space</span>
        </button>
        <button class="mbtn" onclick={copyTerminalCwd}>
          <span class="mbtn-icon">
            <Clipboard size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Copy path to CWD</span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={showTerminalCwd}>
          <span class="mbtn-icon">
            <FolderOpen size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Show Dir</span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={graphTerminalCwd}>
          <span class="mbtn-icon">
            <Network size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Graph dir</span>
          <span class="mbtn-chord"></span>
        </button>
        <div class="msep" role="separator"></div>
        <button class="mbtn" onclick={openFind}>
          <span class="mbtn-icon">
            <Search size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Find</span>
          <span class="mbtn-chord">{chordFor("app.find.open") ?? ""}</span>
        </button>
        <button class="mbtn" onclick={copyScrollback}>
          <span class="mbtn-icon">
            <Clipboard size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Copy Scrollback</span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={() => void restart()}>
          <span class="mbtn-icon">
            <RotateCcw size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Restart</span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={openNewTerminal}>
          <span class="mbtn-icon">
            <TerminalIcon size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">New Terminal</span>
          <span class="mbtn-chord">{chordFor("app.terminal.toggle") ?? ""}</span>
        </button>
        <button class="mbtn" onclick={openNewFile}>
          <span class="mbtn-icon">
            <FilePlus size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">New File</span>
          <span class="mbtn-chord">{chordFor("app.file.new") ?? ""}</span>
        </button>
        <button
          class="mbtn"
          disabled={!canReopenClosedTab()}
          onclick={doReopenClosedTab}
        >
          <span class="mbtn-icon">
            <History size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Reopen Closed Tab</span>
          <span class="mbtn-chord">{chordFor("app.tab.reopenClosed") ?? ""}</span>
        </button>
        {#if splitsAllowed}
          <button class="mbtn" onclick={() => splitPane("row")}>
            <span class="mbtn-icon">
              <SquareSplitHorizontal size={16} strokeWidth={1.75} aria-hidden="true" />
            </span>
            <span class="mbtn-label">Split Right</span>
            <span class="mbtn-chord"></span>
          </button>
          <button class="mbtn" onclick={() => splitPane("column")}>
            <span class="mbtn-icon">
              <SquareSplitVertical size={16} strokeWidth={1.75} aria-hidden="true" />
            </span>
            <span class="mbtn-label">Split Down</span>
            <span class="mbtn-chord"></span>
          </button>
        {/if}
        <button class="mbtn" onclick={openSearch}>
          <span class="mbtn-icon">
            <Search size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Search</span>
          <span class="mbtn-chord">{chordFor("app.search.toggle") ?? ""}</span>
        </button>
        <button class="mbtn" onclick={openSettingsFromMenu}>
          <span class="mbtn-icon">
            <Settings size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Settings</span>
          <span class="mbtn-chord">{chordFor("app.settings.toggle") ?? ""}</span>
        </button>
        <div class="msep" role="separator"></div>
        <div class="mcp-env-row">
          <button class="mbtn" class:on={mcpEnvOn} onclick={toggleMcpEnv}>
            <span class="mbtn-icon">
              {#if mcpEnvOn}
                <Check size={15} strokeWidth={2} aria-hidden="true" />
              {/if}
            </span>
            <span class="mbtn-label">Set MCP env vars</span>
          </button>
          <button
            type="button"
            class="info-btn"
            aria-label="About MCP env vars"
            aria-expanded={mcpInfoOpen}
            onclick={() => (mcpInfoOpen = !mcpInfoOpen)}
          >
            <Info size={15} strokeWidth={1.75} aria-hidden="true" />
          </button>
        </div>
        {#if mcpInfoOpen}
          <div class="mcp-info">
            When on, chan sets CHAN_MCP_SOCKET, CHAN_MCP_SERVER_JSON, and friends in the
            PTY env so external agent CLIs can discover the chan MCP server
            automatically. Turn this off to launch a vanilla shell. Applies to new
            sessions only.
          </div>
        {/if}
        <button class="mbtn" disabled={showMcpEnvDisabled} onclick={showMcpEnv}>
          <span class="mbtn-icon"></span>
          <span class="mbtn-label">Show MCP env in terminal</span>
        </button>
        <div class="msep" role="separator"></div>
        <button class="mbtn" class:on={tab.broadcastEnabled} onclick={toggleBroadcast}>
          <span class="mbtn-icon">
            <Radio size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">
            {tab.broadcastEnabled ? "Broadcast Input On" : "Broadcast Input Off"}
          </span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={toggleAllBroadcastTargets}>
          <span class="mbtn-icon"></span>
          <span class="mbtn-label">
            {allBroadcastTargetsSelected ? "Deselect All" : "Select All"}
          </span>
          <span class="mbtn-chord"></span>
        </button>
        {#if broadcastTargets.length === 0}
          <div class="empty-targets">No other terminal tabs</div>
        {:else}
          {#each broadcastTargets as target (target.id)}
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
  {#if findOpen}
    <div class="terminal-find" role="search" aria-label="find in terminal">
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
    </div>
  {/if}
  {#if tab.richPrompt?.open}
    {#if tab.watcher}
      <BubbleOverlay
        watcher={tab.watcher}
        sessionId={tab.terminalSessionId}
        onRefresh={refreshWatcherEvents}
        onWatcherDetached={watcherDetached}
        onOpenTerminal={(event) => {
          focusTerminalSession(event.session);
          focusTerminalName(event.tab_label ?? event.from);
        }}
      />
    {/if}
    <TerminalRichPrompt
      prompt={tab.richPrompt}
      onSubmit={submitRichPrompt}
      onClose={closeRichPrompt}
      terminalSessionId={tab.terminalSessionId}
      watcherPath={tab.watcher?.path ?? null}
      onWatcherStarted={watcherStarted}
      onWatcherStopped={watcherStopped}
      onSpawned={spawnCreated}
    />
  {/if}
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
  .terminal-find {
    position: absolute;
    top: 8px;
    right: 10px;
    z-index: 2;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.22);
    padding: 5px;
  }
  .session-note {
    color: var(--warn-text);
    font-size: 12px;
    white-space: nowrap;
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
  .terminal-status-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 8px 4px;
    min-width: 0;
  }
  .terminal-status {
    color: var(--text-secondary);
    font-size: 12px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .terminal-status.connected {
    color: var(--accent);
  }
  .env-stale-row {
    margin: 2px 8px 6px;
    padding: 7px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-card);
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.35;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: 6px;
    align-items: center;
  }
  .env-stale-row button {
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--btn-bg);
    color: var(--text);
    font: inherit;
    font-size: 12px;
    padding: 3px 6px;
    cursor: pointer;
    white-space: nowrap;
  }
  .env-stale-row button:hover {
    border-color: var(--btn-hover);
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
  .mbtn:disabled {
    color: var(--text-secondary);
    cursor: not-allowed;
    opacity: 0.58;
  }
  .mbtn:disabled:hover {
    background: none;
  }
  .mbtn-icon {
    width: 18px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .mcp-env-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
  }
  .info-btn {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .info-btn:hover,
  .info-btn[aria-expanded="true"] {
    background: var(--hover-bg);
    color: var(--text);
  }
  .mcp-info {
    margin: 2px 8px 6px 34px;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.35;
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
    .terminal-find { right: 6px; }
    .find {
      width: 112px;
    }
  }
</style>
