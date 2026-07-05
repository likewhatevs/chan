// Pure mechanics for assigning a keyboard shortcut to a command: turn a
// keydown into a candidate chord, and detect conflicts against the
// resolved keymap. Both operate on chord strings and command ids, so
// they are independent of how (and per which OS) the override layer
// persists its chords; the reactive store in keymapOverrides.svelte.ts
// supplies the resolved-keymap entries and owns persistence.

import { chordFromEvent, chordsEqual, type Chord } from "./shortcuts";

/// One command's resolved chord on the platform + OS the caller is
/// assigning for: the user override if one exists, else the built-in.
/// The store builds these across every assignable command and every
/// SHORTCUTS entry so a rebind can collide with a chordless command,
/// an editor chord, or a terminal chord alike.
export type KeymapEntry = { id: string; chord: Chord };

/// Capture a candidate chord from a rebinding keydown. Returns the
/// chord in the registry's grammar (`"Mod+J"`, `"Mod+Shift+K"`), or
/// `null` while the keystroke is not yet a bindable chord: a
/// modifier-only press (still composing) or a bare key with no
/// modifier. Requiring a modifier keeps a plain letter from shadowing
/// ordinary typing once bound, matching the registry's chorded set.
export function captureChord(e: KeyboardEvent): Chord | null {
  return chordFromEvent(e);
}

/// Every entry whose chord matches `candidate`, excluding the command
/// being assigned (rebinding a command to the chord it already has is
/// not a conflict). Empty array means the candidate is free to assign.
/// Chord matching is modifier-alias aware via `chordsEqual`.
export function keymapConflicts(
  candidate: Chord,
  entries: readonly KeymapEntry[],
  excludeId: string,
): KeymapEntry[] {
  return entries.filter(
    (entry) => entry.id !== excludeId && chordsEqual(entry.chord, candidate),
  );
}
