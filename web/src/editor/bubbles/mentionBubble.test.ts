import { describe, expect, test } from "vitest";
import client from "../../api/client.ts?raw";
import contact from "./contact.ts?raw";
import wysiwyg from "../Wysiwyg.svelte?raw";

// `fullstack-a-70`: editor mention completion gap closed.
// SPA wires `api.mentions(q, limit)` against the
// `systacean-35` `/api/mentions` endpoint + merges the
// returned tokens into the contact bubble in mention mode.

describe("fullstack-a-70: api.mentions client method", () => {
  test("api.mentions hits /api/mentions with the q+limit query string", () => {
    expect(client).toMatch(
      /mentions: \(q = "", limit = 10\) => \{[\s\S]*?qs\.set\("limit", String\(limit\)\);[\s\S]*?req<Array<\{ label: string \}>>\([\s\S]*?"GET",[\s\S]*?`\/api\/mentions\?\$\{qs\.toString\(\)\}`/,
    );
  });

  test("doc comment cross-references systacean-35 + the @@ sigil composition", () => {
    expect(client).toMatch(/`fullstack-a-70`/);
    expect(client).toMatch(/`systacean-35`/);
    expect(client).toMatch(/Labels[\s\S]{1,40}arrive WITH the `@@` sigil/i);
  });
});

describe("fullstack-a-70: contact bubble merges mention corpus", () => {
  test("`Suggestion` discriminated union covers contact + mention", () => {
    expect(contact).toMatch(
      /type Suggestion =[\s\S]*?\| \{ kind: "contact"; contact: Contact \}[\s\S]*?\| \{ kind: "mention"; mention: MentionHit \};/,
    );
  });

  test("phase-13 A4: mention corpus surfaced under BOTH triggers", () => {
    // The single-`@` (wiki) trigger now also fetches the mention
    // corpus so `@name` surfaces `@@<Name>` handles, not just
    // contact files. Insertion shape still follows the picked
    // row's kind, not the trigger.
    expect(contact).toMatch(
      /const includeMentions = true;/,
    );
  });

  test("fan-out queries contacts + mentions in parallel", () => {
    expect(contact).toMatch(
      /const contactsP = api\.contacts\(query, PAGE_LIMIT\);[\s\S]*?const mentionsP = includeMentions[\s\S]*?api\.mentions\(query, PAGE_LIMIT\)\.catch\(\(\) => \[\] as MentionHit\[\]\)[\s\S]*?Promise\.all\(\[contactsP, mentionsP\]\)/,
    );
  });

  test("mergeSuggestions dedups mention tokens against contact basename + aliases", () => {
    expect(contact).toMatch(
      /function mergeSuggestions\([\s\S]*?const seen = new Set<string>\(\);[\s\S]*?for \(const c of contactRows\) \{[\s\S]*?seen\.add\(basenameStem\(c\.path\)\);[\s\S]*?for \(const a of c\.aliases\) seen\.add\(a\.toLowerCase\(\)\);/,
    );
  });

  test("commitMention inserts the @@<Name> token verbatim", () => {
    expect(contact).toMatch(
      /function commitMention\(m: MentionHit\): void \{[\s\S]*?opts\.view\.dispatch\(\{[\s\S]*?insert: m\.label/,
    );
  });

  test("Enter key routes contact vs mention to the right commit path", () => {
    expect(contact).toMatch(
      /if \(event\.key === "Enter"\) \{[\s\S]*?if \(hit\.kind === "contact"\) commit\(hit\.contact\);[\s\S]*?else commitMention\(hit\.mention\);/,
    );
  });

  test("mention-only row gets the dim class", () => {
    expect(contact).toMatch(
      /if \(hit\.kind === "mention"\) \{[\s\S]*?row\.classList\.add\("md-bubble-row-mention-only"\);/,
    );
  });
});

describe("fullstack-a-70: Wysiwyg CSS dims mention-only rows", () => {
  test(".md-bubble-row-mention-only opacity rule present", () => {
    expect(wysiwyg).toMatch(
      /:global\(\.md-bubble \.md-bubble-row-mention-only\) \{[\s\S]*?opacity: 0\.7;/,
    );
  });

  test("selected mention-only row restores full opacity", () => {
    expect(wysiwyg).toMatch(
      /:global\(\.md-bubble \.md-bubble-row-mention-only\.md-bubble-row-selected\) \{[\s\S]*?opacity: 1;/,
    );
  });
});
