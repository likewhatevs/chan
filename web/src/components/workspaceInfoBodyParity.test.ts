import { describe, expect, test } from "vitest";
import workspaceInfo from "./WorkspaceInfoBody.svelte?raw";
import carousel from "./EmptyPaneCarousel.svelte?raw";
import graphPanel from "./GraphPanel.svelte?raw";
import fbSurface from "./FileBrowserSurface.svelte?raw";

// Workspace-root inspector behaves like any other directory. The
// `inspector` variant renders the standard directory action row; the
// `dashboard` variant renders the Notes-directories config. Source-level
// pins lock the variant split, action row, config gating, and host wiring.

describe("WorkspaceInfoBody variant split + directory action row", () => {
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

  test("the Notes-directories section carries the divider", () => {
    // A dashed top border separates COCOMO from Notes directories,
    // matching the COCOMO divider idiom.
    expect(workspaceInfo).toMatch(
      /<section class="refs notes-dirs">[\s\S]*?<h4>Notes directories<\/h4>/,
    );
    expect(workspaceInfo).toMatch(
      /\.notes-dirs \{[\s\S]*?border-top: 1px dashed var\(--border\);/,
    );
  });

  test("EmptyPaneCarousel passes variant=\"dashboard\"", () => {
    expect(carousel).toMatch(/<WorkspaceInfoBody[\s\S]*?variant="dashboard"/);
  });

  test("GraphPanel passes onReveal + onSetAsScope for the workspace root", () => {
    expect(graphPanel).toMatch(
      /<WorkspaceInfoBody[\s\S]*?onReveal=\{\(\) => revealPathInBrowserTab\("", true\)\}[\s\S]*?onSetAsScope=\{\(\) => graphFromHere\("", true\)\}/,
    );
  });
});

// Workspace inspector parity with FileInfoBody: clickable Languages
// (graph-opening <button>) and a Contacts section derived from the
// shared semantic graph snapshot.

describe("clickable Languages in the workspace inspector", () => {
  test("an onLanguageClick prop is declared, defaulting to the store helper", () => {
    expect(workspaceInfo).toMatch(
      /onLanguageClick\?\: \(language: string\) => void;/,
    );
    expect(workspaceInfo).toMatch(/onLanguageClick = openGraphForLanguage/);
  });

  test("each language row renders a <button> that fires onLanguageClick", () => {
    // Mirrors FileInfoBody's language rows: a <button> rather than a
    // plain <span>, wired to onLanguageClick.
    expect(workspaceInfo).toMatch(
      /<button[\s\S]*?class="lang-name"[\s\S]*?title="open in graph \(scoped to this language\)"[\s\S]*?onclick=\{\(\) => onLanguageClick\(lang\.name\)\}/,
    );
    expect(workspaceInfo).not.toMatch(
      /<span class="lang-name" title=\{lang\.name\}>/,
    );
  });

  test("all three mount sites pass onLanguageClick={openGraphForLanguage}", () => {
    for (const src of [carousel, graphPanel, fbSurface]) {
      expect(src).toMatch(/onLanguageClick=\{openGraphForLanguage\}/);
    }
  });
});

describe("Contacts section in the workspace inspector", () => {
  test("an onContactNavigate prop is declared", () => {
    expect(workspaceInfo).toMatch(
      /onContactNavigate\?\: \(path: string\) => void;/,
    );
  });

  test("contactPills derive from the shared graph snapshot", () => {
    // Source: graphData.view.nodes, filtered to resolved contact files
    // (node_kind === "contact") + unresolved @@name mention nodes.
    expect(workspaceInfo).toMatch(/const contactPills = \$derived\.by/);
    expect(workspaceInfo).toContain("graphData.view");
    expect(workspaceInfo).toMatch(/n\.node_kind === "contact"/);
    expect(workspaceInfo).toMatch(/n\.kind === "mention"/);
    // Resolved contacts route through onContactNavigate (with the store
    // helper as fallback); unresolved mentions open the node in-graph.
    expect(workspaceInfo).toContain("openGraphForContact");
    expect(workspaceInfo).toContain("openGraphAtNode");
    // The graph is loaded so the section can populate.
    expect(workspaceInfo).toContain("ensureGraphLoaded");
  });

  test("a Contacts section renders the contact pills as clickable refs", () => {
    expect(workspaceInfo).toMatch(
      /<h4>Contacts<\/h4>[\s\S]*?contactPills as c \(c\.key\)[\s\S]*?class="ref contact"[\s\S]*?onclick=\{c\.onClick\}/,
    );
  });

  test("all three mount sites pass onContactNavigate={openGraphForContact}", () => {
    for (const src of [carousel, graphPanel, fbSurface]) {
      expect(src).toMatch(/onContactNavigate=\{openGraphForContact\}/);
    }
  });
});
