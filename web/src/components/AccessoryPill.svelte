<script lang="ts">
  // Floating navigation pill: Files / Search / Assistant / Graph /
  // Settings. The single visual entry point for jumping between
  // window-level overlays from anywhere this is mounted. Callers
  // rely on the global keybindings still firing whether or not the
  // pill is visible (Cmd/Ctrl+P, Cmd/Ctrl+Shift+G, Cmd/Ctrl+H,
  // Cmd/Ctrl+Shift+E, Cmd/Ctrl+,); the pill just makes the
  // affordance discoverable.
  //
  // Layout (top → bottom along the z stack):
  //   - .pill-chrome: rounded background containing the four side
  //     icons (Files / Search / Graph / Settings) plus a transparent
  //     `.enso-slot` reserving 62px of horizontal space in the row's
  //     centre. The chrome owns the bg / border / shadow.
  //   - .fbtn.enso: positioned absolutely above the chrome, centered
  //     horizontally, bottom flush with the chrome's top so the logo
  //     "perches" above the pill rather than expanding it.
  //
  // The side icons collapse to width:0 + opacity:0 while idle (see
  // BottomPill.svelte for the hover-expand cascade), so the chrome
  // shrinks down to just the enso slot when the user isn't over the
  // bar — and unfurls outward on hover.

  import { Folder, Search, Share2, Settings } from "lucide-svelte";
  import {
    assistantOverlay,
    browserOverlay,
    drive,
    openAssistant,
    openBrowser,
    openGraph,
    openSettings,
    searchPanel,
    settingsDisabled,
  } from "../state/store.svelte";
  import { chordFor } from "../state/shortcuts";

  /// Tooltip helper. Pulls the chord from the central shortcuts
  /// registry so the pill's labels stay aligned with App.svelte's
  /// keymap (and the empty-pane / `chan serve --help` tables) instead
  /// of carrying hand-written chord strings that silently drift.
  function tip(label: string, id: string): string {
    const c = chordFor(id);
    return c ? `${label} (${c})` : label;
  }

  /// Master switch state. When off we keep the button visible but
  /// inert + greyed so the entry point stays discoverable and the
  /// tooltip points the user at the toggle in Settings. Hiding the
  /// button made the missing affordance unexplainable, especially
  /// for users who hit Cmd/Ctrl+P and saw nothing happen.
  const assistantEnabled = $derived(
    drive.info?.preferences.assistant.enabled ?? true,
  );

  /// Server-controlled lockdown of the Settings panel (tunnel mode
  /// with --tunnel-public). We grey the button rather than hide it
  /// so a returning owner sees the entry point and understands why
  /// it's inert: matches the issue #21 viewer / kiosk story.
  const settingsLocked = settingsDisabled;
</script>

<div class="pill-stack">
  <div class="pill-chrome">
    <button
      class="fbtn side left"
      class:on={browserOverlay.open}
      title={tip("Files", "app.files.toggle")}
      aria-label="Files"
      onclick={openBrowser}
    >
      <Folder size={19} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <button
      class="fbtn side left"
      title={tip("Search", "app.search.toggle")}
      aria-label="Search"
      onclick={() => (searchPanel.open = true)}
    >
      <Search size={19} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <!-- Reserves horizontal space for the enso so the side icons
         stay symmetric around the center even though the enso is
         no longer in the same flex container. -->
    <div class="enso-slot" aria-hidden="true"></div>
    <button
      class="fbtn side right"
      title={tip("Graph", "app.graph.toggle")}
      aria-label="Graph"
      onclick={openGraph}
    >
      <Share2 size={19} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <button
      class="fbtn side right"
      class:disabled={settingsLocked}
      title={settingsLocked
        ? "Settings disabled while this drive is shared via a tunnel"
        : tip("Settings", "app.settings.toggle")}
      aria-label="Settings"
      aria-disabled={settingsLocked}
      disabled={settingsLocked}
      onclick={openSettings}
    >
      <Settings size={19} strokeWidth={1.75} aria-hidden="true" />
    </button>
  </div>
  <button
    class="fbtn enso"
    class:on={assistantOverlay.open && assistantEnabled}
    class:disabled={!assistantEnabled}
    title={assistantEnabled
      ? tip("Assistant", "app.assistant.toggle")
      : "Assistant is off — enable it in Settings"}
    aria-label="Assistant"
    aria-disabled={!assistantEnabled}
    disabled={!assistantEnabled}
    onclick={openAssistant}
  >
    <span class="enso-mark" aria-hidden="true"></span>
  </button>
</div>

<style>
  /* Anchor for the absolutely-positioned enso. Sized to its inline
     children, so the bottom pill can flex around it. */
  .pill-stack {
    position: relative;
    display: inline-flex;
    align-items: center;
  }
  /* Rounded chrome surrounding the side buttons. Invisible at
     idle (no bg, no border, no shadow) so only the ensō is on
     screen when the bar isn't being interacted with. Hover on the
     ensō flips .bottom-pill:hover on, which fades the chrome in
     and unfurls the sides (see BottomPill.svelte). The shown bg
     reuses --hover-bg so the chrome reads as a translucent halo
     the same shade as the ensō's idle backdrop. */
  .pill-chrome {
    display: flex;
    align-items: center;
    padding: 5px 10px;
    background: transparent;
    box-shadow: none;
    border-radius: 999px;
    transition:
      background 220ms ease,
      box-shadow 220ms ease;
  }
  /* Invisible placeholder reserving horizontal room for the enso
     button so the four side icons stay symmetric around the center
     (two on each side). Matches the enso button width + a 4px gap
     on each side. */
  .enso-slot {
    width: 62px;
    height: 38px;
    flex-shrink: 0;
    margin: 0 4px;
  }
  .fbtn {
    min-width: 38px;
    height: 38px;
    text-align: center;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 19px;
    color: var(--text);
    cursor: pointer;
    font: inherit;
    padding: 0 9px;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .fbtn:hover { background: var(--hover-bg); }
  .fbtn.on {
    background: var(--hover-bg);
    border-color: var(--btn-hover);
  }
  /* Locked Settings entry: keep the affordance visible so the owner
     can still see where it lives, but kill hover feedback and the
     pointer hint so it reads as inert. The native `disabled`
     attribute already blocks onclick; this is purely visual. */
  .fbtn.disabled,
  .fbtn.disabled:hover {
    opacity: 0.35;
    background: transparent;
    cursor: not-allowed;
  }
  .fbtn :global(svg) {
    display: block;
  }
  /* The ensō uses the same chan-mark.png artwork as the empty-pane
     watermark, painted via CSS mask so the silhouette can take the
     theme accent (brand orange, matches chan.app).
     Positioned ABSOLUTELY at the centre of the .pill-stack so it
     sits visually IN FRONT of the chrome (depth, not height). The
     chrome's rounded background still spans the chord-style pill,
     but the logo paints on top so the pill bg does NOT surround
     the ensō. Because the logo is taller than the chrome it pokes
     a few px above and below — that's the intended overhang.
     z-index keeps the logo above .pill-chrome regardless of source
     order. */
  .fbtn.enso {
    position: absolute;
    top: 50%;
    left: 50%;
    z-index: 2;
    width: 62px;
    height: 62px;
    min-width: 62px;
    border-radius: 50%;
    padding: 0;
    /* Always-on translucent shade behind the logo. The pill chrome
       carries the same --hover-bg value, and we want the ensō to
       read as slightly DARKER than the chrome — so it pops out as
       a distinct circle when the bar is expanded. Trick: stack two
       --hover-bg layers via linear-gradient so the effective alpha
       roughly doubles (theme-agnostic, no hard-coded rgba). When
       the chrome isn't visible (idle state), the same stack still
       paints over the page bg so the ensō stays discoverable. */
    background: linear-gradient(var(--hover-bg), var(--hover-bg));
    /* translate(-50%, -50%) centers exactly on the pill's centre.
       The X portion lives in every rule that touches transform so
       the hover wobble doesn't yank the centering. */
    transform: translate(-50%, -50%);
    transition: transform 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .fbtn.enso:hover {
    transform: translate(-50%, -50%) scale(1.06);
  }
  @media (prefers-reduced-motion: reduce) {
    .fbtn.enso,
    .fbtn.enso:hover {
      transition: none;
      transform: translate(-50%, -50%);
    }
  }
  /* Mark sized to nearly fill the enso button so the logo dominates
     the row visually. */
  .fbtn.enso .enso-mark {
    width: 56px;
    height: 56px;
    background-color: var(--assistant-accent);
    -webkit-mask: url('/chan-mark.png') center / contain no-repeat;
            mask: url('/chan-mark.png') center / contain no-repeat;
  }
  .fbtn.enso.on {
    border-color: var(--assistant-accent);
    background: rgba(229, 140, 77, 0.12);
  }
</style>
