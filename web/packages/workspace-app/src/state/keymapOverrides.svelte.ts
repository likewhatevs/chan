// The persisted per-OS keyboard-override layer. A per-command table of
// user-assigned chords, keyed command id -> {web, macos, linux, windows},
// that augments the compile-time SHORTCUTS. Resolution is client-platform
// driven: chan-desktop resolves the slot for its native OS, a browser
// resolves the web slot. The desktop table is a local-machine config part
// applied client-side to every devserver it reaches; a devserver's own
// config carries a web set for that devserver's browser clients.
//
// This module installs the resolver shortcuts.ts calls (registerOverride-
// Resolver) so chordFor and every chord reader see overrides first, and it
// owns the reverse dispatch lookup and the resolved-keymap builder that
// conflict detection consumes. shortcuts.ts stays free of this module so
// the shortcuts-table generator can compile it standalone.

import {
  chordsEqual,
  currentOS,
  currentPlatform,
  osChord,
  registerOverrideResolver,
  SHORTCUTS,
  type Chord,
  type OS,
  type Platform,
} from "./shortcuts";
import type { KeymapEntry } from "./keymapAssign";
import type { Command } from "./commands";

/// The per-OS chord slots stored for one command. `web` is the browser
/// set; the rest are the chan-desktop native sets per OS. All optional:
/// a command carries only the slots the user has assigned.
export type OverrideSlot = "web" | "macos" | "linux" | "windows";
export type CommandOverride = Partial<Record<OverrideSlot, Chord>>;
/// The wire shape round-tripped through the config (command id -> slots).
export type KeymapOverridesWire = Record<string, CommandOverride>;

const keymapOverrides = $state<{ byId: KeymapOverridesWire }>({ byId: {} });

/// Map the running client (platform + OS) to the override slot it reads:
/// a browser uses `web`; chan-desktop uses its native OS slot. This is the
/// whole of the client-platform precedence.
function slotFor(platform: Platform, os: OS): OverrideSlot {
  if (platform === "web") return "web";
  return os === "mac" ? "macos" : os;
}

/// The slot the current client resolves + assigns against.
export function currentSlot(): OverrideSlot {
  return slotFor(currentPlatform(), currentOS());
}

/// The resolver shortcuts.ts consults: the override chord for `id` on the
/// requested client, or undefined when the command has none there.
function resolveOverride(
  id: string,
  platform: Platform,
  os: OS,
): Chord | undefined {
  return keymapOverrides.byId[id]?.[slotFor(platform, os)];
}

registerOverrideResolver(resolveOverride);

/// The override chord for `id` on the current client, or undefined. The
/// launcher reads this to show the assigned chord and to offer a clear.
export function overrideChordFor(id: string): Chord | undefined {
  return resolveOverride(id, currentPlatform(), currentOS());
}

/// The full resolved keymap for the current client: every command's active
/// chord, override first then the SHORTCUTS baseline. Spans every SHORTCUTS
/// entry (so a rebind can't collide with an editor or terminal chord) and
/// every catalog command (so it can't collide with a chordless command that
/// carries an override). Conflict detection runs against this set.
export function resolvedKeymapEntries(
  commands: readonly Command[],
): KeymapEntry[] {
  const platform = currentPlatform();
  const os = currentOS();
  const slot = slotFor(platform, os);
  const byId = new Map<string, Chord>();
  for (const s of SHORTCUTS) {
    const chord = keymapOverrides.byId[s.id]?.[slot] ?? osChord(s, platform, os);
    if (chord) byId.set(s.id, chord);
  }
  for (const c of commands) {
    if (byId.has(c.id)) continue;
    const chord = keymapOverrides.byId[c.id]?.[slot];
    if (chord) byId.set(c.id, chord);
  }
  return [...byId].map(([id, chord]) => ({ id, chord }));
}

/// Reverse lookup for the key dispatch: the command id an OVERRIDE chord
/// fires on the current client, or undefined. Only user-assigned overrides
/// match here (the compile-time onWindowKey branches already fire the
/// built-in chords), and an override equal to the command's own built-in
/// chord is skipped so the default branch and this path never double-fire.
export function commandIdForChord(chord: Chord): string | undefined {
  const platform = currentPlatform();
  const os = currentOS();
  const slot = slotFor(platform, os);
  for (const [id, override] of Object.entries(keymapOverrides.byId)) {
    const oc = override[slot];
    if (!oc || !chordsEqual(oc, chord)) continue;
    const s = SHORTCUTS.find((x) => x.id === id);
    const builtin = s ? osChord(s, platform, os) : undefined;
    if (builtin && chordsEqual(builtin, chord)) continue;
    return id;
  }
  return undefined;
}

// ---- persistence seam --------------------------------------------------
//
// The wire table round-trips through the config; its exact placement is the
// server config-shape note (the desktop table travels client-side, a
// devserver holds a web set for its browser clients). The config layer
// installs the persist writer; until it does, assignment is in-memory.

let persistOverrides: ((wire: KeymapOverridesWire) => void) | null = null;

export function registerOverridePersist(
  fn: ((wire: KeymapOverridesWire) => void) | null,
): void {
  persistOverrides = fn;
}

function persist(): void {
  persistOverrides?.(serializeOverrides());
}

// ---- mutation ----------------------------------------------------------

/// Assign `chord` to `id` for `slot` (the current client's slot by
/// default) and persist. Replaces any existing chord in that slot.
export function assignOverride(
  id: string,
  chord: Chord,
  slot: OverrideSlot = currentSlot(),
): void {
  keymapOverrides.byId[id] = {
    ...(keymapOverrides.byId[id] ?? {}),
    [slot]: chord,
  };
  persist();
}

/// Remove `id`'s override for `slot` (dropping the command entirely when no
/// slot remains) and persist. A no-op when nothing is assigned there.
export function clearOverride(
  id: string,
  slot: OverrideSlot = currentSlot(),
): void {
  const current = keymapOverrides.byId[id];
  if (!current || current[slot] === undefined) return;
  const next: CommandOverride = { ...current };
  delete next[slot];
  if (Object.keys(next).length === 0) delete keymapOverrides.byId[id];
  else keymapOverrides.byId[id] = next;
  persist();
}

// ---- wire round-trip ---------------------------------------------------

/// Snapshot the override table in wire shape for persistence.
export function serializeOverrides(): KeymapOverridesWire {
  const out: KeymapOverridesWire = {};
  for (const [id, override] of Object.entries(keymapOverrides.byId)) {
    out[id] = { ...override };
  }
  return out;
}

/// Replace the in-memory table from a config payload. Unknown slots are
/// dropped and empty commands skipped so a malformed or older config can't
/// seed junk into the resolver.
export function hydrateOverrides(
  wire: KeymapOverridesWire | undefined | null,
): void {
  for (const id of Object.keys(keymapOverrides.byId)) {
    delete keymapOverrides.byId[id];
  }
  if (!wire) return;
  const slots: OverrideSlot[] = ["web", "macos", "linux", "windows"];
  for (const [id, override] of Object.entries(wire)) {
    const clean: CommandOverride = {};
    for (const slot of slots) {
      const chord = override?.[slot];
      if (typeof chord === "string" && chord) clean[slot] = chord;
    }
    if (Object.keys(clean).length > 0) keymapOverrides.byId[id] = clean;
  }
}
