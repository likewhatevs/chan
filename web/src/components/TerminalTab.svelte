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
    Network,
    Pencil,
    Radio,
    RotateCcw,
    Search,
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
    closeTab,
    clearTerminalSession,
    dismissTerminalEnvNamePrompt,
    layout,
    markTerminalEnvNameRestarted,
    openTerminalInActivePane,
    registerTerminalCloseSink,
    registerTerminalInputSink,
    renameTerminalTab,
    reopenClosedTab,
    setTerminalBroadcastEnabled,
    setTerminalBroadcastTarget,
    setTerminalActivity,
    setTerminalMcpEnv,
    setTerminalSession,
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
    revealPathInBrowser,
    scheduleSessionSave,
    ui,
  } from "../state/store.svelte";
  import { terminalWsPath } from "../terminal/session";
  import { handleTerminalMetaKey } from "../terminal/keymap";
  import { injectShowMcpEnvCommand } from "../terminal/mcpEnv";
  import { AGENT_SUBMIT_CHORD } from "../terminal/submitMode";
  import {
    clampScrollbackMb,
    scrollbackLinesFromMb,
    SCROLLBACK_MB_DEFAULT,
  } from "../terminal/scrollback";
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
  // `fullstack-b-11`: scrollback line cap captured at construction
  // time from the persisted MB budget so xterm.js gets a stable
  // number. Held on the component so the "copy scrollback" actions
  // can serialize the same window that's actually in memory rather
  // than the pre-fix 20k constant.
  let scrollbackLines = scrollbackLinesFromMb(SCROLLBACK_MB_DEFAULT);
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
  // `fullstack-a-31`: self appears at the top of the broadcast
  // target list with a "self" marker. Checking the self row sets
  // `broadcastEnabled` (this tab joins the broadcast group);
  // other rows route to `setTerminalBroadcastTarget`. The
  // umbrella "Broadcast Input On/Off" button is gone — the self
  // row is the only knob that controls THIS tab's participation.
  const broadcastTargets = $derived(
    allTerminalTabs().sort((a, b) => {
      if (a.id === tab.id) return -1;
      if (b.id === tab.id) return 1;
      return 0;
    }),
  );
  const selectedBroadcastTargets = $derived(new Set(terminalBroadcastMemberIds(tab)));
  // `fullstack-a-31`: "Select All" walks every row INCLUDING self
  // so the bulk action stays consistent with the per-row UI.
  const allBroadcastTargetsSelected = $derived(
    broadcastTargets.length > 0 &&
      broadcastTargets.every((target) =>
        target.id === tab.id ? tab.broadcastEnabled : selectedBroadcastTargets.has(target.id),
      ),
  );
  const mcpEnvOn = $derived(terminalMcpEnvEnabled(tab));
  const watcherPath = $derived(tab.watcher?.path ?? null);
  /// `fullstack-a-4`: count of survey/poke bubbles currently
  /// visible in the BubbleOverlay (replies are siblings that
  /// don't render). Passed to `TerminalRichPrompt` so the rich
  /// prompt skips auto-focusing its input when bubbles are
  /// waiting — numbered keystrokes then flow to BubbleOverlay's
  /// window keydown handler. Re-derives on every watcher refresh.
  const bubbleCount = $derived(
    (tab.watcher?.events ?? []).filter((event) => event.type !== "survey-reply").length,
  );
  const showMcpEnvDisabled = $derived(tab.sessionMcpEnv === false);
  const staleEnvName = $derived(terminalEnvTabNameStale(tab));
  const showStaleEnvPrompt = $derived(
    staleEnvName && !tab.terminalEnvNamePromptDismissed,
  );
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
    queueMicrotask(() => {
      // `fullstack-a-17`: when the rich prompt is open it owns the
      // keyboard. Cmd+K p on a pane without a terminal spawns one and
      // opens the rich prompt in the same Svelte tick; without this
      // gate, xterm's mount-time focus races past the rich prompt's
      // focus effect and steals the caret. Bump `focusNonce` so the
      // rich prompt's open-effect re-runs and lands the caret on the
      // editor — covers both the Cmd+K p race AND the user returning
      // to a pane whose rich prompt was already open (no focusNonce
      // bump would otherwise fire there).
      if (tab.richPrompt?.open) {
        if (tab.richPrompt) {
          tab.richPrompt.focusNonce = (tab.richPrompt.focusNonce ?? 0) + 1;
        }
        return;
      }
      term?.focus();
    });
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

  // `fullstack-78`: track both the global theme AND the pane-local
  // override so xterm.js' theme (rendered to its own canvas — CSS
  // cascade doesn't reach inside) re-applies on per-pane flip.
  $effect(() => {
    ui.theme;
    const node = layout.nodes[paneId];
    if (node?.kind === "leaf") {
      void node.theme;
    }
    applyTerminalTheme();
  });

  function effectivePaneTheme(): "dark" | "light" {
    const node = layout.nodes[paneId];
    if (node?.kind === "leaf" && node.theme) return node.theme;
    return ui.theme;
  }

  function terminalTheme() {
    // Read CSS variables from `host` (inside the pane) rather than
    // `document.documentElement` so the `.pane[data-theme="..."]`
    // cascade from `-59` resolves to per-pane overrides.
    const styles = getComputedStyle(host ?? document.documentElement);
    const bg = styles.getPropertyValue("--bg").trim() || "#1c1c1e";
    const text = styles.getPropertyValue("--text").trim() || "#ebebf0";
    const cursor = styles.getPropertyValue("--link").trim() || "#58a6ff";
    const base = {
      background: bg,
      foreground: text,
      cursor,
      selectionBackground: "rgba(88, 166, 255, 0.35)",
    };
    const effective = effectivePaneTheme();
    if (effective === "light") {
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
    // `fullstack-b-11`: scrollback honors the Settings MB budget.
    // Read once here so a settings change after spawn doesn't reach
    // through and resize the existing xterm.js buffer; the hint copy
    // under the slider names this spawn-time-only contract.
    scrollbackLines = scrollbackLinesFromMb(
      clampScrollbackMb(drive.info?.preferences?.terminal?.scrollback_mb),
    );
    // `fullstack-b-2`: lineHeight bumped from 1.0 to 1.2 so
    // multi-row ASCII glyphs (e.g. the Claude Code splash cube,
    // figlet output, nethack tiles) render with the row separation
    // a user gets from iTerm. xterm.js's 1.0 default packs ascender
    // glyphs against the next row's descenders; visible regression
    // captured in `docs/journals/phase-8/attachments/image-{3,4}.png`.
    // `fullstack-b-12`: visual parity with iTerm2's defaults.
    // Source Code Pro Regular at 14 pt is bundled with chan
    // (rust-embed via /static/fonts) so the family resolves to a
    // known face on every install; the fallback chain catches the
    // case where the @font-face load is still in flight or the
    // browser declines woff2. Cursor goes to a non-blinking block,
    // matching iTerm's defaults captured in the task spec.
    term = new Terminal({
      allowTransparency: false,
      cursorBlink: false,
      cursorStyle: "block",
      fontFamily:
        '"Source Code Pro", "SF Mono", SFMono-Regular, ui-monospace, Menlo, Consolas, "Liberation Mono", monospace',
      fontSize: 14,
      lineHeight: 1.2,
      macOptionIsMeta: true,
      scrollback: scrollbackLines,
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
        message:
          "The shell in this terminal will be killed and a fresh one started in its place. Any running command will be terminated.",
        confirmLabel: "Restart",
        destructive: true,
      });
      if (!confirmed) return;
    }
    if (tab.controlledTerminal && tab.terminalSessionId) {
      try {
        await api.restartTerminal(tab.terminalSessionId, {
          name: terminalTabName(tab),
          window_id: sessionWindowId(),
        });
        markTerminalEnvNameRestarted(tab);
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
    const text = serialize?.serialize({ scrollback: scrollbackLines }) ?? "";
    if (!text) return;
    await navigator.clipboard?.writeText(text);
    term?.focus();
  }

  async function copySelectionOrScrollback(): Promise<void> {
    closeTabMenu();
    const text =
      term?.getSelection() ||
      serialize?.serialize({ scrollback: scrollbackLines }) ||
      "";
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

  // `fullstack-42` dropped the "Show Dir" and "Graph dir" menu
  // entries. Their click handlers (`showTerminalCwd` /
  // `graphTerminalCwd`) lived here; they came back as dead code so
  // the section is gone. `fullstack-43`'s context-aware spawn will
  // re-read the terminal's CWD through a centralised helper.

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
    // `fullstack-b-13`: when the prompt is in Agent submit-mode,
    // strip any trailing newline the editor left on the buffer
    // and append the agent-submit chord. Claude Code v2.1.145
    // reads `\x1b[27;9;13~` (xterm modifyOtherKeys Cmd+Enter)
    // as submit; a stray `\n` before the chord would land as a
    // newline in the agent's multi-line draft. Shell mode keeps
    // the buffer byte-for-byte (today's behaviour: the shell
    // sees the editor's trailing `\n` as Enter).
    if (tab.richPrompt?.submitMode === "agent") {
      const stripped = source.replace(/\n+$/, "");
      sendUserInput(stripped + AGENT_SUBMIT_CHORD);
    } else {
      sendUserInput(source);
    }
    scheduleTerminalSessionSave();
    // `fullstack-a-4`: caret stays in the rich prompt after
    // Cmd+Enter so consecutive prompts are fluid. Previously we
    // refocused the terminal here, which forced the user to
    // click back into the prompt for every entry.
    if (tab.richPrompt) tab.richPrompt.focusNonce = (tab.richPrompt.focusNonce ?? 0) + 1;
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
    if (!tab.terminalSessionId) return;
    tab.watcher.loading = true;
    try {
      const events = await readWatcherEvents(tab.terminalSessionId);
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
      if (target.id === tab.id) {
        setTerminalBroadcastEnabled(tab, select);
      } else {
        setTerminalBroadcastTarget(tab, target.id, select);
      }
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
        <!-- `fullstack-50`: dropped the "Rich prompt" hamburger
             entry; Cmd+K p is the canonical entry, the rich prompt's
             own `×` button is the exit. Alt+Space still works as the
             out-of-Pane-Mode shortcut for muscle-memory. -->
        <button class="mbtn" onclick={copyTerminalCwd}>
          <span class="mbtn-icon">
            <Clipboard size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Copy path to CWD</span>
          <span class="mbtn-chord"></span>
        </button>
        <!-- `fullstack-42`: dropped "Show Dir" and "Graph dir";
             Pane Mode + context-aware spawn (`fullstack-43`) covers
             both via Cmd+K 2 and Cmd+K 3, with the terminal's CWD
             as the context. -->

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
        <!-- `fullstack-a-31`: per-tab broadcast selector. Drops the
             umbrella "Broadcast Input On/Off" rocker — the per-row
             checkboxes are the only controls. Self appears at the
             top of the list with a "self" marker; checking self
             enrolls this tab in the broadcast group. The container
             label below names the surface so the menu reads
             "broadcast input on/off" as @@Alex spelled it. -->
        <div class="broadcast-section-label">
          <span class="mbtn-icon">
            <Radio size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span>broadcast input on/off</span>
        </div>
        <button class="mbtn" onclick={toggleAllBroadcastTargets}>
          <span class="mbtn-icon"></span>
          <span class="mbtn-label">
            {allBroadcastTargetsSelected ? "Deselect All" : "Select All"}
          </span>
          <span class="mbtn-chord"></span>
        </button>
        {#each broadcastTargets as target (target.id)}
          {@const isSelf = target.id === tab.id}
          {@const isChecked = isSelf
            ? tab.broadcastEnabled
            : selectedBroadcastTargets.has(target.id)}
          <label class="target-row">
            <span class="target-check">
              <input
                type="checkbox"
                checked={isChecked}
                onchange={(e) => {
                  const next = (e.currentTarget as HTMLInputElement).checked;
                  if (isSelf) {
                    setTerminalBroadcastEnabled(tab, next);
                  } else {
                    setTerminalBroadcastTarget(tab, target.id, next);
                  }
                }}
              />
              {#if isChecked}
                <Check size={13} strokeWidth={2} aria-hidden="true" />
              {/if}
            </span>
            <span class="target-name">
              {terminalTabName(target)}
              {#if isSelf}
                <span class="target-self">(self)</span>
              {/if}
            </span>
          </label>
        {/each}
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
      {bubbleCount}
    />
  {/if}
  <!-- `fullstack-a-4`: when the rich prompt is open we reserve
       space at the bottom of the terminal-host equal to the
       prompt's current height plus the resize-handle gap. The
       xterm ResizeObserver picks the new size up and calls
       `fit()`, so the bottom-most rendered line stays visible
       above the prompt instead of being painted over.

       `fullstack-a-29`: prefer the prompt's measured runtime
       height (written by a ResizeObserver in TerminalRichPrompt)
       over the user-resized `heightPx` so the reactor tracks the
       `fullstack-a-24` collapse transition. When collapsed the
       CSS `height: auto` branch shrinks the prompt to header-
       only (~44 px) but `heightPx` stays at the expanded value;
       reading `measuredHeightPx` collapses the reserved space
       in lockstep with the visible pill. Falls back to
       `heightPx` for the brief mount window before the first
       observer tick fires. -->
  <div
    class="terminal-host"
    bind:this={host}
    style:margin-bottom={tab.richPrompt?.open
      ? `${(tab.richPrompt.measuredHeightPx ?? tab.richPrompt.heightPx ?? 320) + 12}px`
      : null}
  ></div>
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
  /* `fullstack-a-31`: section label above the broadcast row list.
     Same icon row + secondary text shape as other menu sections;
     the label is informational, not interactive. */
  .broadcast-section-label {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px 4px;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
    text-transform: lowercase;
    letter-spacing: 0.02em;
  }
  .broadcast-section-label .mbtn-icon {
    color: var(--text-secondary);
  }
  .target-self {
    margin-left: 4px;
    color: var(--text-secondary);
    font-size: 11px;
    font-style: italic;
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
