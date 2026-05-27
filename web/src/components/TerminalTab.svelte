<script lang="ts">
  import { tick } from "svelte";
  import {
    Check,
    Clipboard,
    ClipboardPaste,
    FilePlus,
    Folder,
    History,
    Info,
    MessageSquare,
    Network,
    Pencil,
    Radio,
    RotateCcw,
    Search,
    Settings2,
    Terminal as TerminalIcon,
    X,
  } from "lucide-svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { SearchAddon } from "@xterm/addon-search";
  import { SerializeAddon } from "@xterm/addon-serialize";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import { WebglAddon } from "@xterm/addon-webgl";
  import "@xterm/xterm/css/xterm.css";
  import { api, sessionWindowId, withTokenQuery } from "../api/client";
  import type { TerminalSpawnResponse } from "../api/types";
  import { chordFor, shouldEscapeTerminal } from "../state/shortcuts";
  import {
    advanceTerminalSeq,
    allTerminalTabs,
    broadcastTerminalInput,
    canReopenClosedTab,
    closeTab,
    clearTerminalSession,
    flipHybrid,
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
    tabFocusPulse,
    terminalBroadcastMemberIds,
    terminalEnvTabNameStale,
    terminalMcpEnvEnabled,
    terminalTabName,
    type TerminalTab as TerminalTabState,
  } from "../state/tabs.svelte";
  import {
    workspace,
    effectiveHybridSurfaceTheme,
    fileOps,
    openFsGraphForDirectory,
    scheduleSessionSave,
    setTransientStatus,
    surfaceThemeOverride,
    ui,
  } from "../state/store.svelte";
  import { terminalWsPath } from "../terminal/session";
  import {
    createTerminalKeyboardProtocolState,
    handleTerminalMetaKey,
    installKeyboardProtocolHandlers,
    resetTerminalKeyboardProtocolState,
  } from "../terminal/keymap";
  import { injectShowMcpEnvCommand } from "../terminal/mcpEnv";
  import { installTerminalReportGuards } from "../terminal/xtermReports";
  import { AGENT_SUBMIT_CHORD } from "../terminal/submitMode";
  import {
    clampScrollbackMb,
    scrollbackLinesFromMb,
    SCROLLBACK_MB_DEFAULT,
  } from "../terminal/scrollback";
  import { uiConfirm } from "../state/confirm.svelte";
  import { clampMenu } from "./menuClamp";
  import { portal } from "./portal";
  import {
    closeTabMenu,
    openTabMenu,
    tabMenu,
  } from "../state/tabMenu.svelte";
  import BubbleOverlay from "./BubbleOverlay.svelte";
  import McpEnvInfoModal from "./McpEnvInfoModal.svelte";
  import TerminalRichPrompt from "./TerminalRichPrompt.svelte";
  import { readWatcherEvents } from "../state/watcherEvents";
  import type {
    RichPromptCloseResponse,
    RichPromptResponse,
    RichPromptSubmitResponse,
  } from "../api/types";

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
    | { type: "ready"; cols: number; rows: number; cwd?: string | null; cwd_rel?: string | null }
    | {
        type: "session";
        id: string;
        seq: number;
        missed_bytes?: number;
        bytes_since_focus?: number;
      }
    | { type: "activity"; bytes_since_focus: number }
    | { type: "cwd"; cwd?: string | null; cwd_rel?: string | null }
    | { type: "resize"; cols: number; rows: number }
    | { type: "resize_other"; cols: number; rows: number }
    | { type: "closed"; reason: CloseReason }
    | { type: "exit"; code: number }
    | { type: "error"; message?: string; reason?: string }
    /// `fullstack-a-92`: server-side `dispatch_agent_event` no
    /// longer writes the `poke + chord` echo directly to the
    /// agent session's PTY. It emits this frame instead; the
    /// SPA routes the payload through `sendUserInput` so the
    /// existing broadcast layer (`-a-31`) fans the echo to
    /// every selected broadcast target. Payload is base64 of
    /// the raw bytes (chord may include non-UTF8 bytes per
    /// `fullstack-b-13`'s submit-mode chord; base64 round-trips
    /// the whole sequence without escape-string contortions).
    | {
        type: "agent_event_echo";
        seq: number;
        event_id: string;
        payload_b64: string;
      };

  type CloseReason = "idle" | "workspace" | "shutdown" | "explicit" | "capped" | "error";

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
  let terminalCwdVirtual: string | null = $state(null);
  let watcherPollTimer: ReturnType<typeof setInterval> | null = null;
  let richPromptWorkspaceRequest: Promise<void> | null = null;
  let richPromptWorkspaceCreateKey = "";
  let richPromptWorkspaceStatusKey = "";
  const outputDecoder = new TextDecoder();
  let webglRendererActive = false;
  let webglContextLossRetries = 0;
  let ptyOutputWriteDepth = 0;
  let hostResumeTimers: ReturnType<typeof setTimeout>[] = [];
  const keyboardProtocol = createTerminalKeyboardProtocolState();
  let hostResumeListenerCleanup: (() => void) | null = null;
  // `lane-c addendum-2 item 2`: wall-clock-gap sleep/wake detector. See
  // installHostResumeListeners for why focus/pageshow/visibilitychange
  // miss a macOS display/system sleep in WKWebView.
  let wakeProbeTimer: ReturnType<typeof setInterval> | null = null;
  let lastWakeProbe = 0;
  // Probe every 2s; a gap past 6s (several missed ticks) means JS timers
  // froze - the machine slept - and the probe is firing late on wake.
  const WAKE_PROBE_MS = 2000;
  const WAKE_GAP_MS = 6000;
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
    const unregisterClose = registerTerminalCloseSink(tab.id, closeTerminalForTab);
    return () => {
      unregisterInput();
      unregisterClose();
    };
  });

  $effect(() => {
    const rp = tab.richPrompt;
    const session = tab.terminalSessionId;
    if (!rp || !session) return;
    if (!rp.open && !rp.workspaceName) return;
    if (!rp.workspaceName) {
      const key = `${tab.id}:${session}`;
      if (richPromptWorkspaceCreateKey === key) return;
      richPromptWorkspaceCreateKey = key;
      void ensureRichPromptWorkspace(session);
      return;
    }
    const key = `${rp.workspaceName}:${session}`;
    if (richPromptWorkspaceStatusKey === key) return;
    richPromptWorkspaceStatusKey = key;
    void refreshRichPromptWorkspace(session, rp.workspaceName);
  });

  $effect(() => {
    if (!focused) return;
    // `fullstack-a-64`: read the global tab-focus pulse so this
    // effect re-runs on chord-driven tab switches (Cmd+Shift+[/],
    // Ctrl+Alt+1..9). Without this dep, switching FROM another
    // tab IN to the terminal via chord doesn't pull keyboard
    // focus reliably — the editor's contenteditable retains the
    // DOM focus and the next keystroke damages the doc.
    tabFocusPulse.value;
    queueFit();
    refreshTerminalRenderer();
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

  // `lane-c addendum-1 bug 1`: when focus moves AWAY from this terminal
  // to another pane, the pane losing focus can paint stale in the
  // desktop app's WKWebView - its WebGL renderer leaves the canvas
  // half-updated and a single clear+refresh does not fully correct it.
  // A plain refreshTerminalRenderer() is enough on Blink (web) but not
  // on WebKit, so run the SAME recovery the host-resume / active-flip
  // (Bug 6) paths use - fit + texture-atlas clear + delayed re-fits -
  // on blur too. The size is unchanged on a focus switch, so the fit is
  // a dimensional no-op; the value is the deferred repaint pass WebKit
  // needs. No-op regression on Blink (verified in Chrome); the
  // WKWebView fix is verified by @@Alex in chan-desktop.
  $effect(() => {
    if (focused) return;
    recoverTerminalRendererAfterHostResume();
    sendFocusState();
  });

  // Bug 6: an idle terminal (visible in its pane but NOT focused, or a
  // tab just switched to in a non-active pane) renders garbled until
  // the user clicks or resizes it. The tab uses `visibility: hidden`
  // (not `display: none`) while inactive, so the host keeps layout
  // dimensions, but xterm.js / the WebGL renderer can paint at a stale
  // size (or skip painting) while hidden and there is nothing to force
  // a re-fit + repaint when the tab becomes ACTIVE without also
  // becoming focused. The focus effect (above) covers focus changes
  // and the ResizeObserver covers size changes, but a pure
  // visibility flip on a tab switch hits neither. React to `active`
  // here: when it flips true and the terminal is live, run the same
  // fit + texture-atlas-clear + delayed re-fit recovery used for a
  // host resume, so the terminal converges on its real dimensions and
  // repaints clean. `active` is read first so the effect tracks it;
  // the `term` gate skips the initial mount (start() already fits).
  $effect(() => {
    if (!active) return;
    if (!term) return;
    recoverTerminalRendererAfterHostResume();
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

  // Track the resolved terminal body theme so xterm.js' canvas
  // palette follows the per-surface override.
  $effect(() => {
    effectiveHybridSurfaceTheme("terminal");
    applyTerminalTheme();
  });

  function effectiveTerminalTheme(): "dark" | "light" {
    return effectiveHybridSurfaceTheme("terminal");
  }

  function terminalTheme() {
    // Read CSS variables from `host` so the terminal surface's
    // `data-theme` override resolves before xterm paints.
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
    const effective = effectiveTerminalTheme();
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

  function clearTextureAtlas(): void {
    if (!term || !webglRendererActive) return;
    const maybeClear = (term as Terminal & { clearTextureAtlas?: () => void })
      .clearTextureAtlas;
    maybeClear?.call(term);
  }

  function refreshTerminalRows(): void {
    if (!term) return;
    const maybeRefresh = (term as Terminal & {
      refresh?: (start: number, end: number) => void;
    }).refresh;
    maybeRefresh?.call(term, 0, Math.max(0, term.rows - 1));
  }

  function refreshTerminalRenderer(): void {
    if (!term) return;
    requestAnimationFrame(() => {
      if (!term) return;
      clearTextureAtlas();
      refreshTerminalRows();
    });
    void document.fonts?.ready.then(() => {
      if (!term) return;
      clearTextureAtlas();
      refreshTerminalRows();
    });
  }

  function recoverTerminalRendererAfterHostResume(): void {
    if (!term) return;
    clearHostResumeTimers();
    queueFit();
    refreshTerminalRenderer();
    for (const delay of [50, 250]) {
      const timer = setTimeout(() => {
        hostResumeTimers = hostResumeTimers.filter(
          (candidate) => candidate !== timer,
        );
        queueFit();
        refreshTerminalRenderer();
      }, delay);
      hostResumeTimers.push(timer);
    }
  }

  function clearHostResumeTimers(): void {
    for (const timer of hostResumeTimers) clearTimeout(timer);
    hostResumeTimers = [];
  }

  function installHostResumeListeners(): void {
    if (hostResumeListenerCleanup) return;
    const onHostResume = () => recoverTerminalRendererAfterHostResume();
    const onVisibility = () => {
      if (document.visibilityState === "visible") onHostResume();
    };
    window.addEventListener("focus", onHostResume);
    window.addEventListener("pageshow", onHostResume);
    document.addEventListener("visibilitychange", onVisibility);
    // `lane-c addendum-2 item 2`: macOS screensaver / display + system
    // sleep does NOT fire focus / pageshow / visibilitychange in the
    // desktop app's WKWebView (the window stays "visible" + focused
    // through the sleep), so the listeners above never fire on wake -
    // the WebGL renderer stays glitchy until the user RESIZES a window
    // (ResizeObserver -> queueFit -> recovery; @@Alex's clue). Detect
    // the wake directly: a coarse interval whose callback fires far
    // later than scheduled means the wall clock jumped while JS timers
    // were frozen (the machine slept), so run the same recovery the
    // resize path proves works. One interval per terminal so EVERY pane
    // recovers at once, matching "resize any window clears ALL
    // terminals". (Pure display-only sleep that does not freeze timers
    // is not caught here; verify in chan-desktop - WebKit-only.)
    lastWakeProbe = Date.now();
    wakeProbeTimer = setInterval(() => {
      const now = Date.now();
      const gap = now - lastWakeProbe;
      lastWakeProbe = now;
      if (gap > WAKE_GAP_MS) recoverTerminalRendererAfterHostResume();
    }, WAKE_PROBE_MS);
    hostResumeListenerCleanup = () => {
      window.removeEventListener("focus", onHostResume);
      window.removeEventListener("pageshow", onHostResume);
      document.removeEventListener("visibilitychange", onVisibility);
      if (wakeProbeTimer) {
        clearInterval(wakeProbeTimer);
        wakeProbeTimer = null;
      }
      hostResumeListenerCleanup = null;
    };
  }

  // `webgl-context-loss`: the live WebGL context can be lost long
  // after mount (GPU reset, display sleep, a DPR change when the
  // window moves between Retina and non-Retina displays, tab
  // backgrounding). WKWebView / WebKitGTK (chan-desktop) drop it far
  // more readily than Chrome. The previous handler disposed the
  // renderer and stayed on DOM for the rest of the session, so a
  // single transient loss permanently re-introduced the box-drawing
  // gap bug (`fullstack-b-29`) with no recovery. Recreate the
  // renderer on loss instead, bounded by a small retry budget so a
  // genuinely dead GPU settles on DOM rather than thrashing recreate.
  const WEBGL_MAX_CONTEXT_LOSS_RETRIES = 3;

  function enableWebglRenderer(): void {
    if (!term) return;
    try {
      const webgl = new WebglAddon();
      webgl.onContextLoss(() => {
        webglRendererActive = false;
        webgl.dispose();
        if (term && webglContextLossRetries < WEBGL_MAX_CONTEXT_LOSS_RETRIES) {
          webglContextLossRetries += 1;
          // One [chan] line per budget slot consumed, so a tester
          // watching the webview console (Cmd+Opt+I in chan-desktop)
          // sees each loss + how many recreate attempts remain.
          console.warn(
            `[chan] xterm.js WebGL context lost; recreating renderer (attempt ${webglContextLossRetries}/${WEBGL_MAX_CONTEXT_LOSS_RETRIES}).`,
          );
          // The lost context is not usable synchronously inside the
          // loss callback; recreate on the next frame.
          requestAnimationFrame(() => enableWebglRenderer());
        } else {
          console.warn(
            "[chan] xterm.js WebGL context lost; budget exhausted, staying on the DOM renderer.",
          );
        }
      });
      term.loadAddon(webgl);
      webglRendererActive = true;
      // Repaint so a recreated renderer redraws the visible rows and
      // clears any garbled glyphs left behind by the lost context.
      refreshTerminalRenderer();
    } catch (err) {
      // Tauri webviews effectively always have WebGL; surface the
      // failure for the rare regression case but don't break mount.
      console.warn(
        "[chan] xterm.js WebGL renderer unavailable; falling back to DOM:",
        err,
      );
    }
  }

  function start(): void {
    if (!host || term) return;
    // `fullstack-b-11`: scrollback honors the Settings MB budget.
    // Read once here so a settings change after spawn doesn't reach
    // through and resize the existing xterm.js buffer; the hint copy
    // under the slider names this spawn-time-only contract.
    scrollbackLines = scrollbackLinesFromMb(
      clampScrollbackMb(workspace.info?.preferences?.terminal?.scrollback_mb),
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
    // `fullstack-b-30` slice a: per-OS native-mono default. The
    // chain leads with the OS's installed mono face (SF Mono /
    // Cascadia / DejaVu) so the lean default build (no
    // `--features embed-font`) doesn't 404 on a missing woff2.
    // Source Code Pro stays in the chain but only kicks in when
    // the user opts in via Settings (slice b) — the download flow
    // writes the woff2 to `<user-config>/chan/fonts/` and the SPA
    // reorders the chain to lead with SCP.
    //
    // `fullstack-b-30` slice b: honour the persisted font
    // preference. "source-code-pro" reorders the chain to lead
    // with SCP; the browser still falls back gracefully if the
    // face hasn't loaded yet (or if the user-config-dir copy is
    // missing on a non-embed-font build). "os-default" keeps
    // the slice-a per-OS native lead. Spawn-time-only — mirrors
    // -b-11's scrollback contract; existing terminals keep
    // their current font until session restart.
    const FONT_CHAIN_OS_DEFAULT =
      '"SF Mono", SFMono-Regular, "Cascadia Code", "DejaVu Sans Mono", ui-monospace, Menlo, Consolas, "Liberation Mono", "Source Code Pro", monospace';
    const FONT_CHAIN_SOURCE_CODE_PRO =
      '"Source Code Pro", "SF Mono", SFMono-Regular, "Cascadia Code", "DejaVu Sans Mono", ui-monospace, Menlo, Consolas, "Liberation Mono", monospace';
    const fontPref = workspace.info?.preferences?.terminal?.font ?? "os-default";
    const fontFamily =
      fontPref === "source-code-pro"
        ? FONT_CHAIN_SOURCE_CODE_PRO
        : FONT_CHAIN_OS_DEFAULT;
    term = new Terminal({
      allowTransparency: false,
      cursorBlink: false,
      cursorStyle: "block",
      fontFamily,
      fontSize: 14,
      lineHeight: 1.2,
      macOptionIsMeta: true,
      scrollback: scrollbackLines,
      tabStopWidth: 8,
      theme: terminalTheme(),
    });
    resetTerminalKeyboardProtocolState(keyboardProtocol);
    installTerminalReportGuards(term);
    installKeyboardProtocolHandlers(term, keyboardProtocol, sendInput);
    fit = new FitAddon();
    search = new SearchAddon({ highlightLimit: 1000 });
    serialize = new SerializeAddon();
    term.loadAddon(fit);
    term.loadAddon(search);
    term.loadAddon(serialize);
    term.loadAddon(new WebLinksAddon());
    term.open(host);
    // `fullstack-b-29`: WebGL renderer makes xterm.js's built-in
    // customGlyphs path actually fire — under the default DOM
    // renderer, box-drawing + block-element characters fall
    // through to the system font which (with lineHeight: 1.2)
    // renders with vertical gaps between cells. The WebglAddon
    // draws pixel-perfect glyphs into the cell rectangle
    // including the line-height padding, so ASCII tables +
    // pixel-art mascots render gap-free.
    //
    // WebGL initialisation throws on contexts where the browser
    // declined to allocate a WebGL context (rare on chan-desktop's
    // WKWebView / WebView2, but possible inside headless test
    // harnesses or odd Linux GPU setups), and the live context can
    // later be LOST. enableWebglRenderer() handles both: try/catch on
    // init, then recreate-on-loss (bounded) before settling on the
    // DOM renderer. See the helper for the WKWebView rationale.
    enableWebglRenderer();
    refreshTerminalRenderer();
    installHostResumeListeners();
    term.attachCustomKeyEventHandler(handleTerminalKeyEvent);
    term.onData(handleXtermData);
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
        agentEchoSince: tab.lastAgentEchoSeq,
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
        writePtyOutput(bytes);
        recordOutputBytes(bytes.byteLength);
        maybeRefreshWatcher(bytes);
        maybeSeedPrompt();
        return;
      }
      if (event.data instanceof Blob) {
        const bytes = new Uint8Array(await event.data.arrayBuffer());
        writePtyOutput(bytes);
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
        terminalCwdVirtual = frame.cwd_rel ?? null;
        recoverTerminalRendererAfterHostResume();
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
      } else if (frame.type === "resize" || frame.type === "resize_other") {
        if (!active && term && (term.cols !== frame.cols || term.rows !== frame.rows)) {
          term.resize(frame.cols, frame.rows);
        }
        statusDetail = `${frame.cols}x${frame.rows}`;
      } else if (frame.type === "cwd") {
        terminalCwdAbs = frame.cwd ?? null;
        terminalCwdVirtual = frame.cwd_rel ?? null;
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
      } else if (frame.type === "agent_event_echo") {
        // `fullstack-a-92`: server-side `dispatch_agent_event`
        // (`terminal_sessions.rs`) emits this frame instead of
        // writing the `poke + chord` bytes directly to the
        // PTY. Routing the payload through `sendUserInput`
        // does two things at once:
        //   1. Hits `sendInput` → server writes to the local
        //      PTY (preserving today's "the agent sees `poke`
        //      as if typed" UX).
        //   2. Hits `broadcastTerminalInput` (the existing
        //      `-a-31` fan-out) — when broadcast input is ON
        //      for this session, the same bytes ALSO go to
        //      every selected broadcast target. When OFF, the
        //      fan-out is a no-op + behaviour matches today.
        // Single source of truth on broadcast targeting:
        // `tab.broadcastEnabled` + the broadcast-member set
        // that the SPA already owns.
        const payload = decodeAgentEventEcho(frame.payload_b64);
        if (payload) {
          sendUserInput(payload);
          if (Number.isFinite(frame.seq)) {
            tab.lastAgentEchoSeq = Math.max(
              Math.floor(tab.lastAgentEchoSeq ?? 0),
              Math.floor(frame.seq),
            );
            scheduleTerminalSessionSave();
          }
        }
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

  function writePtyOutput(bytes: Uint8Array): void {
    if (!term) return;
    ptyOutputWriteDepth += 1;
    try {
      term.write(bytes, () => {
        ptyOutputWriteDepth = Math.max(0, ptyOutputWriteDepth - 1);
      });
    } catch (err) {
      ptyOutputWriteDepth = Math.max(0, ptyOutputWriteDepth - 1);
      throw err;
    }
  }

  /// `fullstack-a-92`: decode a base64 agent-event payload
  /// into the string `sendUserInput` expects. Returns null on
  /// malformed base64 so the WS handler can short-circuit
  /// without raising — a malformed echo would still pass the
  /// JSON parse + the type discriminator, so the decoder must
  /// fail soft. The decoded string carries the raw byte
  /// sequence (including any modifyOtherKeys chord that the
  /// server picked per the session's submit-mode); the WS
  /// `input` frame on the inbound leg accepts string verbatim
  /// (PTY write is bytes-of-string).
  function decodeAgentEventEcho(payload_b64: string): string | null {
    try {
      const binary = atob(payload_b64);
      return binary;
    } catch {
      return null;
    }
  }

  function sendUserInput(data: string): void {
    sendInput(data);
    broadcastTerminalInput(tab, data);
  }

  function handleXtermData(data: string): void {
    // xterm emits onData for user input and terminal-generated
    // replies. Replies belong only to the PTY that requested them;
    // they must not enter chan's broadcast fan-out.
    if (ptyOutputWriteDepth > 0) {
      sendInput(data);
      return;
    }
    sendUserInput(data);
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
    // `fullstack-a-93`: trailing-edge fit. ResizeObserver
    // sometimes misses or swallows the FINAL resize event of a
    // drag gesture (browser quirk — observer batches + can
    // collapse intermediate sizes when the host element
    // transitions through layout-thrashing states like
    // `display: none` ↔ visible on tab switch). Without a
    // trailing-edge fit, the terminal stays at the size from
    // the FIRST observed resize tick instead of the FINAL pane
    // width. The rAF above handles the leading edge; the
    // debounced trailing fit below converges on the steady-
    // state size 120ms after the last observed change. Idempotent
    // when the size hasn't drifted: `fit.fit` short-circuits +
    // `term.resize` no-ops on identical cols/rows so no
    // spurious SIGWINCH lands on the PTY.
    scheduleTrailingFit();
  }

  /// `fullstack-a-93`: trailing-edge fit scheduler. Coalesces
  /// rapid ResizeObserver fires (pane-divider drag = dozens per
  /// second) into a single fit 120ms after the last fire. 120ms
  /// matches the perception threshold for "the user has stopped
  /// dragging" + leaves room for one more frame of paint.
  let trailingFitTimer: ReturnType<typeof setTimeout> | null = null;
  function scheduleTrailingFit(): void {
    if (trailingFitTimer) clearTimeout(trailingFitTimer);
    trailingFitTimer = setTimeout(() => {
      trailingFitTimer = null;
      try {
        fit?.fit();
        if (term) statusDetail = `${term.cols}x${term.rows}`;
      } catch {
        // Same throw guard as `queueFit`.
      }
    }, 120);
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
    if (trailingFitTimer) {
      // `fullstack-a-93`: clear the trailing-edge fit timer so a
      // resize-during-teardown rAF doesn't race against the
      // `term?.dispose()` below + throw at fit-time.
      clearTimeout(trailingFitTimer);
      trailingFitTimer = null;
    }
    clearHostResumeTimers();
    hostResumeListenerCleanup?.();
    closeSocket();
    resizeObserver?.disconnect();
    resizeObserver = null;
    term?.dispose();
    term = null;
    ptyOutputWriteDepth = 0;
    webglRendererActive = false;
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

  async function closeTerminalForTab(): Promise<boolean> {
    if (tab.richPrompt?.workspaceName) {
      return discardRichPromptWorkspace();
    }
    explicitCloseSession();
    return true;
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

  /// `fullstack-a-67d`: dropped `doReloadWindow` + `doOpenInspector`
  /// helpers. The Terminal right-click menu no longer carries the
  /// `-b-26` Reload / Open Inspector tail entries per addendum-a's
  /// verbatim spec; Cmd+R + the pane hamburger remain the canonical
  /// surfaces for window-level reload + devtools.

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

  /// `fullstack-a-67d`: From-$CWD spawn entries on the terminal
  /// right-click menu. Each routes through the same
  /// `chan:command` event the keymap layer uses, so the menu
  /// click + the chord both arrive at `runCommand` in
  /// App.svelte. Toggle commands open the surface in a fresh
  /// pane / tab; the originating terminal's $CWD context isn't
  /// passed through (terminal spawn already inherits the
  /// terminal's CWD via the broker, but the FB / Graph toggles
  /// open at the workspace root — accepted deviation, matches the
  /// existing empty-pane spawn-grid behavior).
  function dispatchChanCommand(id: string): void {
    window.dispatchEvent(
      new CustomEvent("chan:command", { detail: { name: id } }),
    );
  }
  function openNewTerminal(): void {
    closeTabMenu();
    dispatchChanCommand("app.terminal.toggle");
  }
  function openNewFileBrowser(): void {
    closeTabMenu();
    dispatchChanCommand("app.files.toggle");
  }
  function openNewGraph(): void {
    closeTabMenu();
    dispatchChanCommand("app.graph.toggle");
  }

  /// `fullstack-a-67d`: Settings (toggle) → flip to the hybrid
  /// back-side config view (HybridTerminalConfig). Mirrors the
  /// pane hamburger's Flip entry.
  function flipToSettings(): void {
    closeTabMenu();
    flipHybrid(paneId);
  }

  /// `fullstack-a-67d`: Close — explicit menu entry per the
  /// addendum spec. `force: true` matches the chord path
  /// (`closeExitedTabFromKey`); the dirty-prompt path lives on
  /// the file editor, not here.
  function closeFromMenu(): void {
    closeTabMenu();
    void closeTab(paneId, tab.id);
  }

  function doReopenClosedTab(): void {
    closeTabMenu();
    reopenClosedTab();
  }

  function requestTerminalCwd(): void {
    send({ type: "cwd" });
  }

  function terminalCwdRel(): string | null {
    if (terminalCwdVirtual !== null) return terminalCwdVirtual;
    const abs = terminalCwdAbs;
    const root = workspace.info?.root;
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

  function richPromptErrorMessage(err: unknown): string {
    return (err as Error)?.message || String(err);
  }

  function applyRichPromptWorkspace(resp: RichPromptResponse): void {
    const rp = tab.richPrompt;
    if (!rp) return;
    rp.phase = resp.phase;
    rp.workspaceName = resp.name;
    rp.draftPath = resp.draft_path;
    rp.workspacePath = resp.workspace_path;
    rp.eventsPath = resp.events_path;
    rp.processPath = resp.process_path;
    rp.workspaceAbs = resp.workspace_abs;
    rp.eventsAbs = resp.events_abs;
    rp.submissionSequence = resp.submission_sequence;
    rp.workspaceError = resp.error ?? null;
    if (resp.watcher.state === "attached") {
      if (tab.watcher?.path !== resp.events_path) watcherStarted(resp.events_path);
    } else {
      if (resp.watcher.state === "failed") {
        rp.workspaceError = resp.error ?? resp.watcher.message;
      }
      watcherStopped();
    }
    scheduleTerminalSessionSave();
  }

  async function ensureRichPromptWorkspace(session: string): Promise<void> {
    if (richPromptWorkspaceRequest) return richPromptWorkspaceRequest;
    richPromptWorkspaceRequest = (async () => {
      const rp = tab.richPrompt;
      if (!rp) return;
      rp.workspaceBusy = true;
      rp.workspaceError = null;
      try {
        const resp = await api.createRichPromptWorkspace(session);
        if (tab.terminalSessionId !== session) return;
        applyRichPromptWorkspace(resp);
      } catch (err) {
        const live = tab.richPrompt;
        if (live) {
          live.phase = "broken";
          live.workspaceError = richPromptErrorMessage(err);
        }
        setTransientStatus(`rich-prompt workspace failed: ${richPromptErrorMessage(err)}`);
      } finally {
        const live = tab.richPrompt;
        if (live) live.workspaceBusy = false;
        richPromptWorkspaceRequest = null;
        scheduleTerminalSessionSave();
      }
    })();
    return richPromptWorkspaceRequest;
  }

  async function refreshRichPromptWorkspace(session: string, name: string): Promise<void> {
    const rp = tab.richPrompt;
    if (!rp) return;
    rp.workspaceBusy = true;
    try {
      const resp = await api.richPromptStatus(name, session);
      if (tab.terminalSessionId !== session || tab.richPrompt?.workspaceName !== name) return;
      applyRichPromptWorkspace(resp);
    } catch (err) {
      const live = tab.richPrompt;
      if (live && live.workspaceName === name) {
        live.phase = "broken";
        live.workspaceError = richPromptErrorMessage(err);
      }
    } finally {
      const live = tab.richPrompt;
      if (live && live.workspaceName === name) live.workspaceBusy = false;
      scheduleTerminalSessionSave();
    }
  }

  function applyRichPromptSubmit(
    resp: RichPromptSubmitResponse,
    bufferAtSubmit: string,
  ): void {
    const rp = tab.richPrompt;
    if (!rp || rp.workspaceName !== resp.name) return;
    rp.phase = resp.phase;
    rp.draftPath = resp.draft_path;
    rp.submissionSequence = resp.submission_sequence;
    rp.workspaceError = null;
    if (rp.buffer === bufferAtSubmit) rp.buffer = "";
    scheduleTerminalSessionSave();
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

  function toggleRichPromptFromMenu(): void {
    closeTabMenu();
    if (tab.richPrompt?.open) closeRichPrompt();
    else openRichPrompt();
  }

  /// `fullstack-a-69`: F-follow-up rewrite. BubbleOverlay
  /// formats the current survey as a markdown quote + calls this
  /// callback to inject it into the Rich Prompt buffer. The
  /// quote is appended (so any in-flight draft survives), a
  /// blank line is added below so the caret lands on a fresh
  /// line, and `focusNonce` is bumped so the Wysiwyg/Source
  /// re-focus and re-mount the new buffer cleanly.
  function quoteIntoRichPrompt(markdown: string): void {
    const rp = ensureRichPrompt();
    const separator = rp.buffer.length === 0 ? "" : "\n\n";
    rp.buffer = `${rp.buffer}${separator}${markdown}\n`;
    rp.open = true;
    rp.focusNonce = (rp.focusNonce ?? 0) + 1;
    scheduleTerminalSessionSave();
  }

  function richPromptUsesAgentSubmit(): boolean {
    const rp = tab.richPrompt;
    if (!rp) return false;
    if (rp.agentTarget && rp.agentTarget !== "none") return true;
    return rp.submitMode === "agent";
  }

  function submitRichPrompt(source: string): void {
    // `fullstack-b-13`: when the prompt is in Agent submit-mode,
    // strip any trailing newline the editor left on the buffer
    // and append the agent-submit chord. Claude Code v2.1.145
    // reads `\x1b[27;9;13~` (xterm modifyOtherKeys Cmd+Enter)
    // as submit; a stray `\n` before the chord would land as a
    // newline in the agent's multi-line draft. Shell mode appends
    // a missing trailing newline so the command actually executes.
    if (richPromptUsesAgentSubmit()) {
      const stripped = source.replace(/\n+$/, "");
      sendUserInput(stripped + AGENT_SUBMIT_CHORD);
    } else {
      sendUserInput(source.endsWith("\n") ? source : `${source}\n`);
    }
    scheduleTerminalSessionSave();
    // `fullstack-a-4`: caret stays in the rich prompt after
    // Cmd+Enter so consecutive prompts are fluid. Previously we
    // refocused the terminal here, which forced the user to
    // click back into the prompt for every entry.
    if (tab.richPrompt) tab.richPrompt.focusNonce = (tab.richPrompt.focusNonce ?? 0) + 1;
    // Archive the exact editor buffer through Core's Rich Prompt
    // workspace. This stays best-effort because the terminal has
    // already received the input.
    void persistRichPromptSubmission(source);
  }

  async function persistRichPromptSubmission(source: string): Promise<void> {
    const trimmed = source.trim();
    if (!trimmed) return;
    const session = tab.terminalSessionId;
    if (!tab.richPrompt?.workspaceName) {
      if (!session) {
        setTransientStatus("rich-prompt workspace not ready");
        return;
      }
      await ensureRichPromptWorkspace(session);
    }
    const rp = tab.richPrompt;
    const name = rp?.workspaceName;
    if (!rp || !name) {
      setTransientStatus("rich-prompt workspace not ready");
      return;
    }
    const bufferAtSubmit = rp.buffer;
    try {
      const resp = await api.submitRichPromptWorkspace(name, {
        content: source,
        expected_sequence: rp.submissionSequence ?? 0,
      });
      applyRichPromptSubmit(resp, bufferAtSubmit);
    } catch (err) {
      if (tab.richPrompt?.workspaceName === name) {
        tab.richPrompt.phase = "broken";
        tab.richPrompt.workspaceError = richPromptErrorMessage(err);
      }
      setTransientStatus(
        `rich-prompt submit archive failed: ${richPromptErrorMessage(err)}`,
      );
    }
  }

  async function discardRichPromptWorkspace(): Promise<boolean> {
    const name = tab.richPrompt?.workspaceName;
    if (!name) {
      explicitCloseSession();
      return true;
    }
    const rp = tab.richPrompt;
    if (rp) {
      rp.workspaceBusy = true;
      rp.workspaceError = null;
    }
    let resp: RichPromptCloseResponse;
    try {
      resp = await api.closeRichPromptWorkspace(name, tab.terminalSessionId ?? "");
    } catch (err) {
      if (tab.richPrompt?.workspaceName === name) {
        tab.richPrompt.phase = "broken";
        tab.richPrompt.workspaceError = richPromptErrorMessage(err);
        tab.richPrompt.workspaceBusy = false;
      }
      setTransientStatus(`rich-prompt close failed: ${richPromptErrorMessage(err)}`);
      scheduleTerminalSessionSave();
      return false;
    }
    if (resp.phase !== "discarded") {
      if (tab.richPrompt?.workspaceName === name) {
        tab.richPrompt.phase = "broken";
        tab.richPrompt.workspaceError = resp.error ?? "rich prompt close failed";
        tab.richPrompt.workspaceBusy = false;
      }
      setTransientStatus(`rich-prompt close failed: ${resp.error ?? "unknown error"}`);
      scheduleTerminalSessionSave();
      return false;
    }
    if (tab.richPrompt?.workspaceName === name) {
      tab.richPrompt.phase = "discarded";
      tab.richPrompt.open = false;
      tab.richPrompt.workspaceBusy = false;
      tab.richPrompt.workspaceError = null;
    }
    tab.watcher = undefined;
    clearTerminalSession(tab);
    scheduleTerminalSessionSave();
    return true;
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
    // `fullstack-a-86`: auto-dismiss the reload-detected
    // "watcher detached" toast — informational; user
    // doesn't need to act on it.
    setTransientStatus("watcher detached on reload");
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
      // systacean-14: server-side may report "terminal watcher is not
      // attached" (HTTP 400) when the SerTab was restored from
      // session storage after a serve restart but the new server has
      // no watcher attached for this session. Mirror BubbleOverlay's
      // detach detection so the pill clears on first refresh instead
      // of leaving a permanent red toast on the UI.
      const raw = (err as Error).message || "";
      if (/409|404|watcher|not found|not attached|conflict/i.test(raw)) {
        watcherDetached();
        return;
      }
      tab.watcher.error = `watch read failed: ${raw || "unknown error"}`;
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
    // `fullstack-a-94`: removed the third Alt+Space handler.
    // `-a-90` swept the two keymap-driven branches but missed
    // THIS one — the xterm `customKeyEventHandler` translation
    // layer registered at line ~424. Caught empirically by
    // @@WebtestA (`aed06ef`); audit-grep needs to include
    // `attachCustomKeyEventHandler` chord paths going forward.
    //
    // `fullstack-a-91`: chord-escape registry. When the
    // incoming event matches a shortcut flagged
    // `escapeTerminal: true` in shortcuts.ts, return false so
    // xterm doesn't consume the keystroke (the contract
    // `attachCustomKeyEventHandler` reads: false = let the
    // browser dispatch it). App.svelte's window-level keymap
    // then handles the chord. Without this gate Cmd+P,
    // Cmd+Shift+M, Cmd+R, etc. fired from a focused terminal
    // would be swallowed by xterm + written to the PTY as
    // escape sequences.
    if (shouldEscapeTerminal(e)) return false;
    return handleTerminalMetaKey(e, sendUserInput, keyboardProtocol);
  }

  function onShellKeydown(e: KeyboardEvent): void {
    if (closeExitedTabFromKey(e)) {
      return;
    }
    // `fullstack-a-90`: removed the legacy `Alt+Space` rich-prompt
    // chord. Cmd+P (native), Cmd+Alt+P (web Mac), and `Mod+. p`
    // (Hybrid Nav) cover the rich-prompt entry points.
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

  /// `fullstack-a-67d` slice 2: open / close the MCP env info
  /// modal. Closing the menu when the modal opens keeps the
  /// chrome from stacking — the modal sits at z=26000 above the
  /// menu bubble, but the bubble visually competes for
  /// attention; collapsing it on open keeps the dialog the only
  /// focus.
  function openMcpInfoModal(): void {
    closeTabMenu();
    mcpInfoOpen = true;
  }
  function closeMcpInfoModal(): void {
    mcpInfoOpen = false;
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
  data-theme={surfaceThemeOverride("terminal")}
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
      use:portal
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
      <!-- `fullstack-a-67d`: status reads "connected: <detail>"
           per addendum-a (colon, not em dash). -->
      <div class="terminal-status-row">
        <span class:connected={status === "connected"} class="terminal-status">
          {status}{statusDetail ? `: ${statusDetail}` : ""}
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
      <!-- `fullstack-a-67d`: menu reshape per addendum-a Terminal
           spec. Header (Name → SEP → status colon → MCP-env +
           Restart) lands ahead of the find/copy band; the new
           "From $CWD" section gathers New File / New Terminal /
           New File Browser / New Graph; the broadcast section
           keeps its slice-1 shape (Terminals dropdown + Jitter
           is deferred to a follow-up — backend gap on Jitter
           persistence); Settings (flipHybrid) + Reopen + Close
           anchor the foot. `Reload Window` and `Open Inspector`
           tail entries (originally added by `-b-26`) dropped per
           the addendum's verbatim spec; Cmd+R and the pane
           hamburger still surface them. -->
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
        <!-- `fullstack-a-67d` slice 2: info button opens a
             modal dialog (McpEnvInfoModal.svelte) per
             addendum-a's "dialog like the New File one" spec.
             The standalone "Show MCP env in terminal" button
             moved INTO the dialog as its primary CTA; the menu
             row now just carries the toggle + the info button. -->
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
            onclick={openMcpInfoModal}
          >
            <Info size={15} strokeWidth={1.75} aria-hidden="true" />
          </button>
        </div>
        <button class="mbtn destructive" onclick={() => void restart()}>
          <span class="mbtn-icon">
            <RotateCcw size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Restart</span>
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
        <button class="mbtn" onclick={copyTerminalCwd}>
          <span class="mbtn-icon">
            <Clipboard size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Copy path to $CWD</span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={copyScrollback}>
          <span class="mbtn-icon">
            <Clipboard size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Copy Scrollback</span>
          <span class="mbtn-chord"></span>
        </button>
        <div class="msep" role="separator"></div>
        <!-- `fullstack-a-67d`: "From $CWD" spawn band. New File
             uses the existing `openNewFile` which seeds the
             dialog with `$CWD/untitled.md`. New Terminal / FB /
             Graph fire the same `chan:command` events the
             chord-routing layer + the empty-pane carousel use,
             so handlers stay singular. -->
        <div class="from-cwd-label">From $CWD</div>
        <button class="mbtn" onclick={openNewFile}>
          <span class="mbtn-icon">
            <FilePlus size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">New File</span>
          <span class="mbtn-chord">{chordFor("app.file.new") ?? ""}</span>
        </button>
        <button class="mbtn" onclick={openNewTerminal}>
          <span class="mbtn-icon">
            <TerminalIcon size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">New Terminal</span>
          <span class="mbtn-chord">{chordFor("app.terminal.toggle") ?? ""}</span>
        </button>
        <button class="mbtn" onclick={openNewFileBrowser}>
          <span class="mbtn-icon">
            <Folder size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">New File Browser</span>
          <span class="mbtn-chord">{chordFor("app.files.toggle") ?? ""}</span>
        </button>
        <button class="mbtn" onclick={openNewGraph}>
          <span class="mbtn-icon">
            <Network size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">New Graph</span>
          <span class="mbtn-chord">{chordFor("app.graph.toggle") ?? ""}</span>
        </button>
        <div class="msep" role="separator"></div>
        <button class="mbtn" onclick={toggleRichPromptFromMenu}>
          <span class="mbtn-icon">
            <MessageSquare size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">
            {tab.richPrompt?.open ? "Hide Rich Prompt" : "Show Rich Prompt"}
          </span>
          <span class="mbtn-chord">
            {chordFor("app.terminal.richPrompt") ?? ""}
          </span>
        </button>
        <div class="msep" role="separator"></div>
        <!-- `fullstack-a-31`: per-tab broadcast selector. Drops
             the umbrella "Broadcast Input On/Off" rocker — the
             per-row checkboxes are the only controls. Self
             appears at the top of the list with a "self"
             marker.
             `fullstack-a-67d`: addendum-a calls for wrapping
             the per-target list inside a "Terminals" expander
             dropdown with a Jitter slider at the top of the
             dropdown. The Jitter persistence + broadcast-delay
             logic is a chan-server gap; scope-poked as a
             follow-up. Section UI kept as-is until the backend
             lands. -->
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
        <div class="msep" role="separator"></div>
        <button class="mbtn" onclick={flipToSettings}>
          <span class="mbtn-icon">
            <Settings2 size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Settings</span>
          <span class="mbtn-chord"></span>
        </button>
        <div class="msep" role="separator"></div>
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
        <button class="mbtn" onclick={closeFromMenu}>
          <span class="mbtn-icon">
            <X size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Close</span>
          <span class="mbtn-chord">{chordFor("app.tab.close") ?? ""}</span>
        </button>
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
        onQuoteToPrompt={(markdown) => quoteIntoRichPrompt(markdown)}
      />
    {/if}
    <TerminalRichPrompt
      prompt={tab.richPrompt}
      onSubmit={submitRichPrompt}
      terminalSessionId={tab.terminalSessionId}
      watcherPath={tab.watcher?.path ?? null}
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

<McpEnvInfoModal
  open={mcpInfoOpen}
  onClose={closeMcpInfoModal}
  onShowInTerminal={showMcpEnv}
  showInTerminalDisabled={showMcpEnvDisabled}
/>

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
    z-index: 25500;
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
    transition: transform 200ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .terminal-tab-menu-bubble:hover {
    transform: scale(1.015);
  }
  @keyframes bubble-pop {
    0% { opacity: 0; transform: scale(0.92); }
    100% { opacity: 1; transform: scale(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .terminal-tab-menu-bubble {
      animation: none;
      transition: none;
    }
    .terminal-tab-menu-bubble:hover {
      transform: none;
    }
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
    transform-origin: left center;
    transition:
      background 80ms ease,
      color 80ms ease,
      transform 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .mbtn:hover,
  .mbtn.on {
    background: var(--hover-bg);
  }
  .mbtn:hover:not(:disabled) {
    transform: scale(1.02);
  }
  .mbtn:disabled {
    color: var(--text-secondary);
    cursor: not-allowed;
    opacity: 0.58;
  }
  .mbtn:disabled:hover {
    background: none;
    transform: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .mbtn {
      transition: background 80ms ease, color 80ms ease;
    }
    .mbtn:hover {
      transform: none;
    }
  }
  /* `fullstack-a-67d`: destructive hint for Restart per
     addendum-a spec. Color-only; no background change so the
     hover affordance still reads. */
  .mbtn.destructive {
    color: var(--danger-text, #d33);
  }
  /* `fullstack-a-67d`: "From $CWD" section label. Subdued
     style matching the .terminal-status row's secondary
     text — telegraphs section grouping, not actionable. */
  .from-cwd-label {
    padding: 4px 8px 2px;
    color: var(--text-secondary);
    font-size: 11px;
    text-transform: lowercase;
    letter-spacing: 0.02em;
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
  .info-btn:hover {
    background: var(--hover-bg);
    color: var(--text);
  }
  /* `fullstack-a-67d` slice 2: dropped `.info-btn[aria-expanded]`
     + `.mcp-info` selectors along with the inline popover; the
     info button now opens McpEnvInfoModal.svelte (modal sits
     at z=26000 above the menu bubble). */
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
