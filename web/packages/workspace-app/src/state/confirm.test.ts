// @vitest-environment jsdom

import { afterEach, describe, expect, test } from "vitest";
import { confirmState, resolveConfirm, uiConfirm } from "./confirm.svelte";

// A focusable node attached to the document, standing in for the caret's
// invoking surface (a terminal or editor).
function focusable(): HTMLButtonElement {
  const el = document.createElement("button");
  document.body.appendChild(el);
  return el;
}

// Stand-in for the modal parking focus on its OK button. uiConfirm does
// not move focus by itself (the ConfirmModal component does, in the app),
// so tests reproduce the modal-open state by focusing this before
// dismissing.
function parkModalFocus(): HTMLButtonElement {
  const ok = focusable();
  ok.focus();
  return ok;
}

afterEach(() => {
  confirmState.open = false;
  confirmState.resolve = null;
  document.body.replaceChildren();
});

describe("confirm dialog focus restoration", () => {
  test("cancel restores focus to the pre-modal element", async () => {
    const origin = focusable();
    origin.focus();
    expect(document.activeElement).toBe(origin);

    const p = uiConfirm({ title: "Discard changes?" });
    const ok = parkModalFocus();
    expect(document.activeElement).toBe(ok);

    resolveConfirm(false);
    await expect(p).resolves.toBe(false);
    expect(document.activeElement).toBe(origin);
  });

  test("accept restores focus when the origin is still connected", async () => {
    const origin = focusable();
    origin.focus();

    const p = uiConfirm({ title: "Overwrite file?" });
    parkModalFocus();

    resolveConfirm(true);
    await expect(p).resolves.toBe(true);
    expect(document.activeElement).toBe(origin);
  });

  test("a disconnected origin is skipped without throwing", async () => {
    const origin = focusable();
    origin.focus();

    const p = uiConfirm({ title: "Close anyway?" });
    // The invoking surface unmounts before the modal resolves (tab close,
    // terminal restart). Removing the focused node drops focus to body.
    origin.remove();
    expect(document.activeElement).toBe(document.body);

    expect(() => resolveConfirm(true)).not.toThrow();
    await expect(p).resolves.toBe(true);
    expect(document.activeElement).toBe(document.body);
  });

  test("a stacked confirm restores the original pre-modal target", async () => {
    const origin = focusable();
    origin.focus();

    const first = uiConfirm({ title: "First" });
    const firstOk = parkModalFocus();
    expect(document.activeElement).toBe(firstOk);

    // A second confirm opens while the first is still up. It must keep the
    // ORIGINAL pre-modal target, not the first modal's OK button.
    const second = uiConfirm({ title: "Second" });
    await expect(first).resolves.toBe(false);
    parkModalFocus();

    resolveConfirm(true);
    await expect(second).resolves.toBe(true);
    expect(document.activeElement).toBe(origin);
  });
});
