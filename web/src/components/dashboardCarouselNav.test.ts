import { describe, expect, test } from "vitest";
import dashboardBack from "./dashboard/DashboardSlotBack.svelte?raw";

// DB1 (@@Host wave-3): the dashboard config back's segmented
// About|Workspace|Search picker becomes a carousel navigator - prev/next
// chevrons + a dot pager + a pause/play toggle - mirroring the front
// carousel's control affordances. Asserted as source shape because the
// markup is a Svelte component, not a pure function; the real interaction
// is browser-smoked.

describe("DB1: dashboard config slot selector is a carousel navigator", () => {
  test("imports the chevron + play/pause icons", () => {
    expect(dashboardBack).toMatch(
      /import \{[^}]*\bChevronLeft\b[^}]*\bChevronRight\b[^}]*\bPause\b[^}]*\bPlay\b[^}]*\} from "lucide-svelte"/,
    );
  });

  test("prev/next chevrons step the shared cursor across all three slots", () => {
    expect(dashboardBack).toMatch(
      /function step\(delta: number\): void \{[\s\S]{1,200}selectSlot\(\(slot \+ delta \+ SLOTS\.length\) % SLOTS\.length\)/,
    );
    expect(dashboardBack).toMatch(
      /class="nav-arrow"[\s\S]{1,160}onclick=\{\(\) => step\(-1\)\}[\s\S]{1,120}<ChevronLeft/,
    );
    expect(dashboardBack).toMatch(
      /class="nav-arrow"[\s\S]{1,160}onclick=\{\(\) => step\(1\)\}[\s\S]{1,120}<ChevronRight/,
    );
  });

  test("one dot per slot, active filled, selecting moves the cursor", () => {
    expect(dashboardBack).toMatch(
      /\{#each SLOTS as label, i \(label\)\}[\s\S]{1,260}class="dot-btn"[\s\S]{1,120}class:active=\{slot === i\}[\s\S]{1,200}onclick=\{\(\) => selectSlot\(i\)\}/,
    );
  });

  test("pause/play toggles the per-tab autoRotate field the front carousel reads", () => {
    expect(dashboardBack).toMatch(
      /const autoRotate = \$derived\(tab\.autoRotate \?\? true\)/,
    );
    expect(dashboardBack).toMatch(
      /function toggleAutoRotate\(\): void \{[\s\S]{1,160}tab\.autoRotate = !autoRotate;/,
    );
    expect(dashboardBack).toMatch(
      /class="cycle-toggle"[\s\S]{1,200}onclick=\{toggleAutoRotate\}[\s\S]{1,300}\{#if autoRotate\}[\s\S]{1,120}<Pause[\s\S]{1,160}<Play/,
    );
  });
});
