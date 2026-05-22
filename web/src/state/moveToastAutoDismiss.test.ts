import { describe, expect, test } from "vitest";
import store from "./store.svelte.ts?raw";

// `fullstack-a-85`: move-success toast should auto-dismiss
// like every other action confirmation. Pre-fix the success
// branch wrote `ui.status =` directly (persistent shape)
// instead of using `setTransientStatus(msg)` which clears at
// `TRANSIENT_STATUS_DEFAULT_MS` (3s).

describe("fullstack-a-85: move success uses setTransientStatus", () => {
  test("success branch routes the moveMsg through setTransientStatus", () => {
    expect(store).toMatch(
      /const moveMsg =[\s\S]*?if \(moveMsg\) \{[\s\S]*?setTransientStatus\(moveMsg\);/,
    );
  });

  test("empty-linkBits path clears ui.status (no orphan 'Moving…')", () => {
    expect(store).toMatch(
      /\} else \{[\s\S]*?No link updates worth surfacing[\s\S]*?ui\.status = null;/,
    );
  });

  test("rename failure path stays persistent (direct ui.status assignment)", () => {
    expect(store).toMatch(
      /\} catch \(e\) \{[\s\S]*?ui\.status = `rename failed: \$\{\(e as Error\)\.message\}`;/,
    );
  });

  test("pre-fix sticky shape gone (no direct ui.status = `Moved...` assignment in success branch)", () => {
    // Pin the absence of the pre-`-a-85` shape:
    //   ui.status = linkBits.length > 0 ? `Moved '${...}' (...)`: null;
    // The new shape uses a `moveMsg` local + setTransientStatus.
    expect(store).not.toMatch(
      /ui\.status =\s*\n\s*linkBits\.length > 0\s*\n\s*\? `Moved/,
    );
  });
});
