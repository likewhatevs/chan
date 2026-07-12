import { describe, expect, test } from "vitest";
import preflight from "./PreflightOverlay.svelte?raw";

// The `cs` terminal-alias offer appears only when `cs` is GENUINELY absent from
// the user's PATH -- the card's correct semantics, independent of host type.
// `csOffer` (the server's `cs_link`) is that signal: the server sets it only
// when its PATH scan finds no `cs`, and chan-desktop now resolves the user's
// real interactive PATH before the embedded server starts, so the scan is
// accurate even on a macOS GUI launch. So the gate is purely `csOffer` (plus
// not-locked, not-dismissed); the old `!isTauriDesktop()` host-suppression
// workaround is gone -- gating on host type wrongly denied the offer to a
// desktop user who genuinely lacks `cs`.
//
// Static `?raw` source-pin (repo convention): pins the gate so a host-type
// suppression can't silently creep back. Real behaviour is browser-smoked.
describe("PreflightOverlay cs offer: gates on real cs presence", () => {
  test("showCsCard gates purely on csOffer, not the host type", () => {
    expect(preflight).toMatch(
      /const showCsCard = \$derived\(!!csOffer && !locked && !csDismissed\);/,
    );
  });

  test("no isTauriDesktop host-suppression remains in the overlay", () => {
    expect(preflight).not.toMatch(/isTauriDesktop/);
  });
});
