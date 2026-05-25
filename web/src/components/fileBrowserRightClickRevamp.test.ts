import { describe, expect, test } from "vitest";
import surface from "./FileBrowserSurface.svelte?raw";
import pane from "./Pane.svelte?raw";

// `fullstack-a-67e`: File Browser right-click / hamburger menu
// revamp per addendum-a's verbatim spec. Slice 1 covers the FB
// tab right-click + hamburger surface (FileBrowserSurface.svelte).
// The in-tree selection menu (FileTree.svelte row right-click)
// is a sibling slice — separate pins land with that change.

describe("fullstack-a-67e: FBSurface menu header — path-derived drive label + path row", () => {
  test("drive-label-row renders the API label without an editable input", () => {
    expect(surface).toMatch(/<li class="drive-label-row" role="none" title=\{drive\.info\?\.root\}>/);
    expect(surface).toMatch(/class="drive-label-text">\{drive\.info\?\.label \?\? ""\}/);
    expect(surface).not.toContain("drive-rename-input");
    expect(surface).not.toContain("commitDriveName");
    expect(surface).not.toContain("api.updatePreferences");
  });

  test("drive-path-row renders with HardDrive icon + click → showDriveInfo + fade-on-overflow", () => {
    expect(surface).toMatch(
      /class="drive-path-row"[\s\S]{1,200}onclick=\{showDriveInfo\}[\s\S]{1,400}<HardDrive size=\{16\}/,
    );
    expect(surface).toMatch(
      /class="drive-path-text">\{drive\.info\?\.root \?\? ""\}/,
    );
    // The fade pattern from `-a-67 slice 1b` (Graph) ports over.
    expect(surface).toMatch(
      /\.drive-path-text\)[\s\S]{1,800}mask-image: linear-gradient\(to right, black calc\(100% - 1\.25rem\), transparent\);/,
    );
  });
});

describe("fullstack-a-67e: FBSurface menu body — dock / expand / reload / import in order", () => {
  test("dock variant can open a File Browser tab for current selection or drive", () => {
    expect(surface).toMatch(
      /function openCurrentInFileBrowser\(\): void \{[\s\S]{1,300}const path = browserSelection\.path;[\s\S]{1,200}const tab = openBrowserInActivePane\(path \? \{ select: path \} : \{\}\);[\s\S]{1,200}tab\.inspectorOpen = true;/,
    );
    expect(surface).toMatch(
      /if \(path\) \{[\s\S]{1,400}tab\.showDrive = false;[\s\S]{1,200}browserSelection\.path = path;[\s\S]{1,200}browserSelection\.showDrive = false;[\s\S]{1,600}tab\.showDrive = true;[\s\S]{1,200}browserSelection\.path = null;[\s\S]{1,200}browserSelection\.showDrive = true;/,
    );
    expect(surface).toMatch(
      /\{#if isDock\}[\s\S]{1,400}onclick=\{openCurrentInFileBrowser\}[\s\S]{1,400}<span class="menu-row-label">Open in File Browser<\/span>/,
    );
  });

  test("dock drive path row matches Open in File Browser for drive details", () => {
    expect(surface).toMatch(
      /function showDriveInfo\(\): void \{[\s\S]{1,120}if \(isDock\) \{[\s\S]{1,120}openCurrentInFileBrowser\(\);[\s\S]{1,120}return;/,
    );
  });

  test("dock toggles come after the SEP that follows the path row", () => {
    expect(surface).toMatch(
      /class="drive-path-text">[\s\S]{1,400}<li class="sep" role="separator"><\/li>[\s\S]{1,400}toggleStick\("left"\)[\s\S]{1,400}toggleStick\("right"\)/,
    );
  });

  test("expand-all + reload sit between dock and import sections", () => {
    expect(surface).toMatch(
      /toggleStick\("right"\)[\s\S]{1,1000}<li class="sep" role="separator"><\/li>[\s\S]{1,200}onclick=\{toggleAll\}[\s\S]{1,1000}onclick=\{reloadTree\}/,
    );
  });

  test("Import contacts entry kept, after reload band", () => {
    expect(surface).toMatch(
      /onclick=\{reloadTree\}[\s\S]{1,800}<li class="sep" role="separator"><\/li>[\s\S]{1,400}onclick=\{openImportContacts\}/,
    );
  });
});

describe("fullstack-a-67e: FBSurface menu foot — Settings / Reopen / Close (tab variant only)", () => {
  test("Settings (flip) entry gated on isTab + onFlip", () => {
    expect(surface).toMatch(
      /\{#if isTab && onFlip\}[\s\S]{1,800}onclick=\{flipToSettings\}/,
    );
  });

  test("flipToSettings routes through onFlip callback", () => {
    expect(surface).toMatch(
      /function flipToSettings\(\): void \{[\s\S]{1,200}menu\?\.close\(\);[\s\S]{1,200}onFlip\?\.\(\);/,
    );
  });

  test("Reopen + Close entries gated on isTab only", () => {
    expect(surface).toMatch(
      /\{#if isTab\}[\s\S]{1,2000}onclick=\{doReopenClosedTab\}[\s\S]{1,1000}onclick=\{closeFromMenu\}/,
    );
  });

  test("closeFromMenu routes through onClose callback", () => {
    expect(surface).toMatch(
      /function closeFromMenu\(\): void \{[\s\S]{1,200}menu\?\.close\(\);[\s\S]{1,200}onClose\?\.\(\);/,
    );
  });
});

describe("fullstack-a-67e: dropped entries", () => {
  test("Rename drive... (modal) entry no longer rendered", () => {
    expect(surface).not.toMatch(/<span class="menu-row-label">Rename drive\.\.\.<\/span>/);
  });

  test("New file / New directory entries no longer in this menu (moved to selection menu)", () => {
    expect(surface).not.toMatch(/<span class="menu-row-label">New file<\/span>/);
    expect(surface).not.toMatch(/<span class="menu-row-label">New directory<\/span>/);
  });

  test("legacy folder-row / folder-label / folder-path CSS classes dropped", () => {
    expect(surface).not.toMatch(/class="folder-row"/);
    expect(surface).not.toMatch(/class="folder-label"/);
  });
});

describe("fullstack-a-67e: Pane.svelte wires onFlip into the tab variant", () => {
  test("Pane passes onFlip={() => flipHybrid(pane.id)} to FileBrowserSurface", () => {
    expect(pane).toMatch(
      /<FileBrowserSurface[\s\S]{1,400}onFlip=\{\(\) => flipHybrid\(pane\.id\)\}/,
    );
  });
});
