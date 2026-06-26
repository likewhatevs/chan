import { describe, expect, test } from "vitest";
import workspaceInfo from "./WorkspaceInfoBody.svelte?raw";
import workspaceSlotConfig from "./dashboard/WorkspaceSlotConfig.svelte?raw";
import carousel from "./EmptyPaneCarousel.svelte?raw";
import graphPanel from "./GraphPanel.svelte?raw";
import fbSurface from "./FileBrowserSurface.svelte?raw";

// Workspace-root inspector behaves like any other directory. The
// `inspector` variant renders the standard directory action row; the
// `dashboard` variant (Dashboard front slide) drops it. The read-only
// recent-workspaces list lives on WorkspaceSlotConfig (the slot's
// flip-back); there is no default-workspace concept, since chan open
// requires an explicit path. Source-level pins lock the variant split,
// action row, the recents' home, and host wiring.

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

  test("the action row is a File Browser pill + secondary dropdown", () => {
    // The row renders the shared split-action pill: a "File Browser"
    // primary action plus the secondary dropdown built from actionModel.
    expect(workspaceInfo).toMatch(
      /<InspectorActionPill[\s\S]*?main=\{actionModel\.main\}[\s\S]*?secondary=\{actionModel\.secondary\}/,
    );
    expect(workspaceInfo).toMatch(
      /main: \{ label: "File Browser", onClick: openRootInBrowser \}/,
    );
  });

  test("the dropdown offers Upload + Download (download disabled while busy)", () => {
    // Upload triggers the hidden picker; onUploadPicked uploads to the
    // workspace root (relative path ""). Download targets the root as a
    // directory (is_dir = true) and disables while a transfer is busy.
    expect(workspaceInfo).toMatch(/label: "Upload", onClick: triggerUpload/);
    expect(workspaceInfo).toMatch(/fileOps\.uploadFilesTo\("", files\)/);
    expect(workspaceInfo).toMatch(
      /label: "Download",\s*onClick: downloadSelection/,
    );
    expect(workspaceInfo).toMatch(
      /fileOps\.downloadPathWithProgress\("", true\)/,
    );
  });

  test("the dropdown offers Terminal from here, rooted at the workspace root", () => {
    // Mirrors FileInfoBody's directory "New terminal here": a terminal
    // rooted at the workspace root (relative path "").
    expect(workspaceInfo).toMatch(
      /label: "Terminal from here", onClick: newTerminalHere/,
    );
    expect(workspaceInfo).toMatch(/terminalFromHereTarget\("", true\)/);
  });

  test("File Browser primary prefers onReveal, else reveals the root", () => {
    // openRootInBrowser mirrors FileInfoBody.openDirInBrowser: call the
    // host's onReveal when present, otherwise reveal the root ("") in the
    // current browser.
    expect(workspaceInfo).toMatch(
      /function openRootInBrowser\(\)[\s\S]*?if \(onReveal\)[\s\S]*?onReveal\(\);[\s\S]*?revealPathInBrowser\("", \{ enter: true, inspectorOpen: true \}\)/,
    );
  });

  test("Graph from here is gated on onSetAsScope inside the action model", () => {
    expect(workspaceInfo).toMatch(
      /if \(onSetAsScope\) \{[\s\S]*?secondary\.push\(\{ label: "Graph from here", onClick: onSetAsScope \}\)/,
    );
  });

  test("the inline download progress indicator is retired (the transfer bubble owns it)", () => {
    // Download progress now shows in the single transfer bubble, not an inline
    // inspector bar — parity with FileInfoBody, which also dropped its `.dl-*`.
    expect(workspaceInfo).not.toContain('class="dl-indicator"');
    expect(workspaceInfo).not.toContain("downloadTransfer");
  });

  test("WorkspaceSlotConfig carries no default-workspace config, only recents", () => {
    // chan open requires an explicit workspace path, so neither
    // WorkspaceInfoBody nor WorkspaceSlotConfig carries a default-root field
    // or its autosave plumbing. WorkspaceSlotConfig's flip-back holds only the
    // read-only recent-workspaces list.
    expect(workspaceInfo).not.toContain("editedDefaultRoot");
    expect(workspaceInfo).not.toContain('class="notes-dirs"');

    expect(workspaceSlotConfig).toMatch(/<h3>Workspaces<\/h3>/);
    expect(workspaceSlotConfig).not.toContain("editedDefaultRoot");
    expect(workspaceSlotConfig).not.toContain("scheduleDefaultRootSave");
    expect(workspaceSlotConfig).not.toContain("default_workspace_root");
    expect(workspaceSlotConfig).toMatch(/globalConfig\?\.workspaces/);
  });

  test("WorkspaceSlotConfig divides chan-reports / Metadata archive / Workspaces", () => {
    // Two dashed separators: one before Metadata archive, one before the
    // Workspaces config, matching the workspace-inspector divider idiom.
    expect(workspaceSlotConfig).toMatch(
      /<section class="divided">\s*<h3>Metadata archive<\/h3>/,
    );
    expect(workspaceSlotConfig).toMatch(
      /<section class="divided">\s*<h3>Workspaces<\/h3>/,
    );
    expect(workspaceSlotConfig).toMatch(
      /\.divided \{[\s\S]*?border-top: 1px dashed var\(--border\);/,
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
