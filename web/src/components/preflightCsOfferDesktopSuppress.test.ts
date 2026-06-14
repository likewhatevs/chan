import { describe, expect, test } from "vitest";
import preflight from "./PreflightOverlay.svelte?raw";

// The non-blocking `cs` terminal-alias offer must never appear under the
// chan-desktop host: the desktop owns the `~/.local/bin/{chan,cs}` shims on
// boot, so the manual offer is redundant and its `ln -s` hint (which points
// into the `.app`/AppImage mount) is wrong for a desktop-owned install. The
// server's `cs_on_path()` suppression misses a macOS GUI launch (restricted
// launchd `$PATH`), so the card is gated on `!isTauriDesktop()` directly.
//
// Static `?raw` source-pin (repo convention, see cmdRWindowReload.test.ts):
// pins the import + the gate so the suppression can't silently regress. The
// real desktop-vs-browser behaviour is browser-smoked separately.
describe("PreflightOverlay cs offer — desktop host suppression", () => {
  test("isTauriDesktop imported from api/desktop", () => {
    expect(preflight).toMatch(
      /import \{[^}]*\bisTauriDesktop\b[^}]*\} from "\.\.\/api\/desktop";/,
    );
  });

  test("showCsCard gates on !isTauriDesktop() alongside the existing conditions", () => {
    expect(preflight).toMatch(
      /const showCsCard = \$derived\(\s*!!csOffer && !locked && !csDismissed && !isTauriDesktop\(\),?\s*\);/,
    );
  });
});
