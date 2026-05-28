import { describe, expect, test } from "vitest";
import inspector from "./InspectorBody.svelte?raw";
import languageBody from "./LanguageInfoBody.svelte?raw";
import panel from "./GraphPanel.svelte?raw";

// Phase-13 A3: language bubbles previously had no inspector. The
// dispatcher fell through to TagInfoBody with a null selection, so
// the panel rendered the empty placeholder. These `?raw` pins lock
// the new language arm + body + GraphPanel wiring at source level
// (the wiring is reactive Svelte, not pure functions).

describe("A3: InspectorBody dispatches a language arm", () => {
  test("InspectorSelection grows a language variant", () => {
    expect(inspector).toMatch(
      /kind: "language";\s*language: string;\s*label: string;\s*files\?: number;\s*code\?: number;/,
    );
  });

  test("the language arm renders LanguageInfoBody with stats + onSetAsScope", () => {
    expect(inspector).toMatch(/import LanguageInfoBody from "\.\/LanguageInfoBody\.svelte";/);
    expect(inspector).toMatch(
      /\{:else if selection\.kind === "language"\}[\s\S]*?<LanguageInfoBody[\s\S]*?language=\{selection\.language\}[\s\S]*?files=\{selection\.files\}[\s\S]*?code=\{selection\.code\}[\s\S]*?\{onSetAsScope\}/,
    );
  });
});

describe("A3: LanguageInfoBody renders name + file/code stats + Graph from here", () => {
  test("the body shows the language chip + title", () => {
    expect(languageBody).toMatch(/<span class="kind-chip language">language<\/span>/);
    expect(languageBody).toMatch(/<h3 class="title" title=\{language\}>\{label\}<\/h3>/);
  });

  test("files + code lines render from props", () => {
    expect(languageBody).toMatch(/files !== undefined[\s\S]*?\{files\}/);
    expect(languageBody).toMatch(/code !== undefined[\s\S]*?\{code\.toLocaleString\(\)\}/);
  });

  test("Graph from here is gated on onSetAsScope", () => {
    expect(languageBody).toMatch(
      /\{#if onSetAsScope\}[\s\S]*?onclick=\{onSetAsScope\}>Graph from here<\/button>/,
    );
  });
});

describe("A3: GraphPanel maps a language node to the language selection", () => {
  test("inspectorSelection has a language branch carrying files + code", () => {
    expect(panel).toMatch(
      /selectedNode\.kind === "language"[\s\S]*?kind: "language",\s*language: selectedNode\.language,\s*label: selectedNode\.label,\s*files: selectedNode\.files,\s*code: selectedNode\.code,/,
    );
  });

  test("Graph from here on a language re-scopes to the language lens", () => {
    expect(panel).toMatch(
      /inspectorSelection\?\.kind === "language"[\s\S]*?rescopeFromHere\(`language:\$\{inspectorSelection\.language\}`\)/,
    );
  });
});
