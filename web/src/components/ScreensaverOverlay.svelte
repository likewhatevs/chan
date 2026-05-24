<script lang="ts">
  // `fullstack-a-77` slice 2: full-window screensaver overlay
  // + PIN entry. Mounted at App root + gated on
  // `screensaver.locked`. While locked:
  //
  // * The entire drive view sits behind an opaque overlay.
  // * PIN input is the only focusable surface (the overlay's
  //   role="dialog" + aria-modal="true" telegraphs that to
  //   ATs).
  // * Esc / clicking the backdrop do NOT dismiss — the only
  //   way out is a correct PIN (or no-PIN drives where the
  //   chan-drive layer returns `verified: false` regardless,
  //   matching the "lockout is moot" framing).
  // * Wrong PIN triggers a 400ms shake + clears the input.
  //   No rate limiting (local-only threat model per the task
  //   body).
  //
  // Mount strategy: rendered at App root (after every other
  // overlay) so the z-index is unambiguous — the screensaver
  // sits over the disconnect overlay, the missing-token
  // overlay, the spawn dialog, etc. If those fire while the
  // user is away they may still surface visibility cues
  // (titlebar / browser notifications) but the SPA itself is
  // covered.

  import { tick } from "svelte";
  import { Lock } from "lucide-svelte";
  import {
    screensaver,
    unlockWithPin,
    unlockWithoutPin,
  } from "../state/screensaver.svelte";
  import { drive } from "../state/store.svelte";
  import MatrixRain from "./screensaver/MatrixRain.svelte";

  let pin = $state("");
  let busy = $state(false);
  let shake = $state(false);
  let errorMessage = $state<string | null>(null);
  let inputEl = $state<HTMLInputElement | undefined>();

  /// `fullstack-a-77`: focus the PIN input the moment the
  /// overlay mounts. Tick first so the contenteditable
  /// transition lands; then focus + select. The no-PIN
  /// branch (`fullstack-a-77c`) has no input element, so
  /// the focus call no-ops there.
  $effect(() => {
    if (!screensaver.locked) return;
    pin = "";
    errorMessage = null;
    void tick().then(() => {
      inputEl?.focus();
      inputEl?.select();
    });
  });

  /// `fullstack-a-77c`: no-PIN branch. Dismiss on any
  /// key / pointer event anywhere on the backdrop. The
  /// `pin_set` gate inside `unlockWithoutPin()` makes
  /// this safe to wire unconditionally — when a PIN is
  /// set the helper bails out + the existing PIN form
  /// owns input.
  function onBackdropKey(e: KeyboardEvent): void {
    if (screensaver.pin_set) return;
    e.preventDefault();
    unlockWithoutPin();
  }
  function onBackdropPointer(): void {
    if (screensaver.pin_set) return;
    unlockWithoutPin();
  }

  async function submit(): Promise<void> {
    if (busy) return;
    if (!pin) {
      errorMessage = "Enter a PIN to unlock";
      return;
    }
    busy = true;
    errorMessage = null;
    const salt = drive.info?.root ?? "";
    const ok = await unlockWithPin(pin, salt);
    busy = false;
    if (!ok) {
      // `fullstack-a-77`: wrong-PIN feedback. Shake the
      // overlay + clear the input. No rate limiting per the
      // task body's local-only framing.
      shake = true;
      errorMessage = screensaver.pin_set
        ? "Wrong PIN"
        : "No PIN set on this drive. Open Settings to configure.";
      pin = "";
      // Reset the shake class after the animation finishes
      // so a subsequent wrong-PIN attempt re-triggers it.
      setTimeout(() => {
        shake = false;
      }, 400);
      void tick().then(() => inputEl?.focus());
    }
  }

  function onKey(e: KeyboardEvent): void {
    if (e.key === "Enter") {
      e.preventDefault();
      void submit();
    }
  }
</script>

{#if screensaver.locked}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="screensaver-backdrop"
    role="dialog"
    aria-modal="true"
    aria-label="Screen locked"
    onkeydown={onBackdropKey}
    onclick={onBackdropPointer}
    tabindex="-1"
  >
    {#if screensaver.theme === "matrix"}
      <MatrixRain />
    {/if}
    <div class="screensaver-card" class:shake>
      <div class="screensaver-icon" aria-hidden="true">
        <Lock size={32} strokeWidth={1.5} />
      </div>
      <h2 class="screensaver-title">Screen locked</h2>
      {#if screensaver.pin_set}
        <p class="screensaver-sub">Enter your PIN to unlock.</p>
        <input
          bind:this={inputEl}
          bind:value={pin}
          type="password"
          class="screensaver-pin"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          disabled={busy}
          onkeydown={onKey}
          placeholder="PIN"
          aria-label="PIN"
        />
        {#if errorMessage}
          <div class="screensaver-error" role="alert">{errorMessage}</div>
        {/if}
        <button
          type="button"
          class="screensaver-unlock"
          onclick={submit}
          disabled={busy}
        >
          {busy ? "Unlocking…" : "Unlock"}
        </button>
      {:else}
        <!-- `fullstack-a-77c`: no-PIN branch. Helper text
             promises "any input unlocks"; the backdrop's
             onkeydown + onclick handlers fulfill it. -->
        <p class="screensaver-sub">
          No PIN set on this drive. Press any key or click to
          unlock.
        </p>
      {/if}
    </div>
  </div>
{/if}

<style>
  /* Full-window cover. z-index above every other chan overlay
     (the spawn dialog sits at z=50 per `-a-78`; team dialog
     same; disconnect / missing-token overlays use 1500+).
     2000 puts the screensaver above all of them. */
  .screensaver-backdrop {
    position: fixed;
    inset: 0;
    z-index: 2000;
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }
  .screensaver-card {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    padding: 2rem 2.5rem;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
    min-width: 320px;
    max-width: 90vw;
  }
  .screensaver-icon {
    color: var(--text-secondary);
  }
  .screensaver-title {
    margin: 0;
    font-size: 1.2rem;
    color: var(--text);
  }
  .screensaver-sub {
    margin: 0;
    font-size: 0.875rem;
    color: var(--text-secondary);
    text-align: center;
    max-width: 28em;
  }
  .screensaver-pin {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 8px 12px;
    color: var(--text);
    font: inherit;
    font-size: 1rem;
    width: 100%;
    text-align: center;
    letter-spacing: 0.3em;
  }
  .screensaver-pin:focus {
    outline: none;
    border-color: var(--accent);
  }
  .screensaver-error {
    color: var(--danger-text, #d33);
    font-size: 0.875rem;
  }
  .screensaver-unlock {
    background: var(--accent);
    color: var(--bg);
    border: 1px solid var(--accent);
    border-radius: 4px;
    padding: 8px 16px;
    cursor: pointer;
    font: inherit;
    min-width: 120px;
  }
  .screensaver-unlock:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  /* `fullstack-a-77`: shake feedback on wrong PIN. 400ms
     duration matches the timer that clears the `shake`
     flag; the CSS animation re-runs on each fresh
     `class:shake` toggle. */
  .screensaver-card.shake {
    animation: screensaver-shake 0.4s ease-in-out;
  }
  @keyframes screensaver-shake {
    0%, 100% { transform: translateX(0); }
    20% { transform: translateX(-8px); }
    40% { transform: translateX(8px); }
    60% { transform: translateX(-6px); }
    80% { transform: translateX(6px); }
  }
</style>
