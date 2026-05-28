import { describe, expect, test } from "vitest";
import workspaceInfo from "./WorkspaceInfoBody.svelte?raw";
import carousel from "./EmptyPaneCarousel.svelte?raw";
import graphPanel from "./GraphPanel.svelte?raw";

// A1 (phase-13 round-1 closing item): the workspace-root inspector now
// behaves "like any other directory". It gains the standard directory
// action row in the default `inspector` variant, and the
// Notes-directories config section is gated to a `dashboard`-only
// variant (the user wants Notes-dirs only in the Dashboard, not the
// inspector). These ?raw source-pattern pins lock the variant split,
// the action row, the config gating, and the host wiring.

describe("A1: WorkspaceInfoBody variant split + directory action row", () => {
  test("a `variant` prop selects inspector vs dashboard", () => {
    // The prop is declared with the two-value union and defaults to
    // "inspector" so legacy callers get the directory-style body.
    expect(workspaceInfo).toMatch(
      /variant\?\: "inspector" \| "dashboard";/,
    );
    expect(workspaceInfo).toMatch(/variant = "inspector"/);
  });

  test("the inspector variant renders the directory action row", () => {
    // The action row is gated on variant === "inspector".
    expect(workspaceInfo).toMatch(/\{#if variant === "inspector"\}/);
  });

  test("the action row renders Upload + Download (download disabled while busy)", () => {
    // Upload triggers the hidden picker; onUploadPicked uploads to the
    // workspace root (relative path ""). Download targets the root as a
    // directory (is_dir = true) and disables while a transfer is busy.
    expect(workspaceInfo).toMatch(/onclick=\{triggerUpload\}/);
    expect(workspaceInfo).toMatch(
      /fileOps\.uploadFilesTo\("", files\)/,
    );
    expect(workspaceInfo).toMatch(/onclick=\{downloadSelection\}/);
    expect(workspaceInfo).toMatch(
      /fileOps\.downloadPathWithProgress\("", true\)/,
    );
    expect(workspaceInfo).toMatch(/disabled=\{downloadBusy\}/);
  });

  test("Show in File Browser is gated on onReveal", () => {
    expect(workspaceInfo).toMatch(
      /\{#if onReveal\}[\s\S]*?onclick=\{onReveal\}[\s\S]*?Show in File Browser/,
    );
  });

  test("Graph from here is gated on onSetAsScope inside the row", () => {
    expect(workspaceInfo).toMatch(
      /\{#if onSetAsScope\}[\s\S]*?onclick=\{onSetAsScope\}[\s\S]*?Graph from here/,
    );
  });

  test("the download progress indicator (.dl-*) is present", () => {
    expect(workspaceInfo).toContain('class="dl-indicator"');
    expect(workspaceInfo).toContain("clearDownloadTransfer");
  });

  test("the Notes-directories section is gated to variant === dashboard", () => {
    // The config section (heading + default-root field + recents) only
    // renders for the Dashboard. The {#if variant === "dashboard"} guard
    // wraps the Notes-directories <section>.
    expect(workspaceInfo).toMatch(
      /\{#if variant === "dashboard"\}[\s\S]*?<h4>Notes directories<\/h4>/,
    );
    // The plumbing is preserved: the default-root field still binds and
    // autosaves.
    expect(workspaceInfo).toContain("bind:value={editedDefaultRoot}");
    expect(workspaceInfo).toContain("scheduleSave");
  });

  test("EmptyPaneCarousel passes variant=\"dashboard\"", () => {
    expect(carousel).toMatch(/<WorkspaceInfoBody variant="dashboard" \/>/);
  });

  test("GraphPanel passes onReveal + onSetAsScope for the workspace root", () => {
    expect(graphPanel).toMatch(
      /<WorkspaceInfoBody[\s\S]*?onReveal=\{\(\) => revealPathInBrowserTab\("", true\)\}[\s\S]*?onSetAsScope=\{\(\) => graphFromHere\("", true\)\}/,
    );
  });
});
