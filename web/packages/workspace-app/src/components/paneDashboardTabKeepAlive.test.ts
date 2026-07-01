import { describe, expect, test } from "vitest";
import pane from "./Pane.svelte?raw";
import dashboardTab from "./DashboardTab.svelte?raw";
import carousel from "./EmptyPaneCarousel.svelte?raw";

// Dashboard tabs are kept ALIVE, exactly like graphs, terminals, and file
// editors (see paneGraphTabKeepAlive): Pane.svelte renders every dashboard
// tab from an each-block inside .face.front and flips an `active` prop;
// inactive dashboards hide via the visibility:hidden contract (never
// display:none — a 0x0 host would make the Indexing GraphCanvas refit to
// nothing and lose its layout). Before this, DashboardTab mounted only the
// active tab from the if-chain, so every switch remounted it and the
// Indexing carousel's GraphCanvas + 3s indexer poll rebuilt from scratch —
// the visible "reload on tab switch". These pins catch a regression back to
// that.

describe("dashboard tabs survive tab switches (keep-alive)", () => {
  test("dashboard each-block renders all dashboard tabs, keyed by tab id", () => {
    expect(pane).toMatch(
      /\{#each pane\.tabs\.filter\(\(t\) => t\.kind === "dashboard"\) as t \(t\.id\)\}\s+<DashboardTab/,
    );
  });

  test("dashboard tabs no longer mount from the active-tab if-chain", () => {
    // The pre-fix branch mounted ONLY the active dashboard
    // (`<DashboardTab tab={active} ...>` under the front-face if-chain), so
    // every switch remounted it and the Indexing graph reloaded. The
    // back-face DashboardSlotBack dispatch still keys off
    // `active?.kind === "dashboard"` — that chain is fine; what must not
    // return is a DashboardTab mounted off `active`.
    expect(pane).not.toMatch(/<DashboardTab\s+tab=\{active\}/);
  });

  test("active prop is gated by !paneMode.active + !pane.showingBack + activeTabId", () => {
    expect(pane).toMatch(
      /<DashboardTab\s+tab=\{t\}\s+active=\{!paneMode\.active && !pane\.showingBack && t\.id === pane\.activeTabId\}/,
    );
  });

  test("no `focused` prop on DashboardTab (a dashboard owns no keyboard caret)", () => {
    expect(pane).not.toMatch(/<DashboardTab\s+tab=\{t\}[\s\S]{1,300}focused=/);
  });
});

describe("DashboardTab threads active + carries the keep-alive contract", () => {
  test("declares an `active` prop (defaulting true for non-pane hosts)", () => {
    expect(dashboardTab).toMatch(/let \{ tab, active = true \}: Props = \$props\(\);/);
    // The old frontActive name is gone.
    expect(dashboardTab).not.toMatch(/frontActive/);
  });

  test("threads active to the carousel so a hidden dashboard pauses + stops polling", () => {
    expect(dashboardTab).toMatch(/<EmptyPaneCarousel[\s\S]{1,200}\{active\}/);
  });

  test("root carries the keep-alive contract: class:active + aria-hidden", () => {
    expect(dashboardTab).toMatch(
      /class="dashboard"\s+class:active\s+aria-label="Dashboard"\s+aria-hidden=\{!active\}/,
    );
  });

  test("hidden dashboards keep layout via visibility, not display:none", () => {
    expect(dashboardTab).toMatch(
      /\.dashboard \{[^}]*position: absolute;[^}]*inset: 0;[^}]*visibility: hidden;[^}]*pointer-events: none;[^}]*\}/,
    );
    expect(dashboardTab).toMatch(
      /\.dashboard\.active \{\s*visibility: visible;\s*pointer-events: auto;\s*\}/,
    );
    // flex:1 is dropped (no longer a flex child of the pane body).
    expect(dashboardTab).not.toMatch(/\.dashboard \{[^}]*flex: 1;[^}]*\}/);
  });
});

describe("EmptyPaneCarousel gates the indexing poll on active", () => {
  test("the indexing refresh effect bails while inactive (no background poll)", () => {
    // A kept-alive but hidden dashboard must not hammer /api/indexing/state.
    expect(carousel).toMatch(
      /if \(slideIndex !== 1 \|\| !active\) return;/,
    );
  });

  test("auto-rotate is paused while inactive", () => {
    expect(carousel).toMatch(/!active \|\| !autoRotate/);
  });
});
