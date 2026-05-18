# webtest-a-6: Round 2 wave-A walkthrough lane (Lane A)

Owner: @@WebtestA
Cut by: @@Architect
Date: 2026-05-18

## Goal

Walk through Round 2 wave-A as it lands. Lane A angle:
the frontend surface — bubble overlay, watcher-set dialog,
survey rendering, terminal-tab status bullet.

This is a rolling task; append verdicts as each piece
lands; ping me after each cluster.

## Relevant links

* Backend: [../systacean/systacean-9.md](../systacean/systacean-9.md).
* Frontend: [../fullstack/fullstack-13.md](../fullstack/fullstack-13.md).
* Schema: [../architect/journal.md](../architect/journal.md)
  ("2026-05-18 21:00 BST" entry).

## Acceptance criteria

For each item below, report PASS / FAIL / PARTIAL with
enough detail for the implementer to act.

### When `systacean-9` lands

1. `POST /api/terminal/<session>/watcher` accepts a JSON
   body with a target dir. Verify via `curl` or chrome
   network panel.
2. Atomic write a synthetic survey event to the watched
   dir (Python `os.replace` or shell `mv` from a temp).
   Confirm the targeted tab receives `poke\n` in its
   PTY.
3. Malformed JSON: doesn't crash chan-server, logged
   warning visible.
4. Unknown `type` field: logged + ignored.

### When `fullstack-13` lands

5. Rich prompt "Watch directory" affordance pulls up the
   new-file dialog, accepts a directory selection, fires
   the API call.
6. Bubble overlay renders over the terminal pane when an
   event lands. Underlying xterm output remains visible.
7. Survey rendering: 1×N variant (single question + 2-3
   options) — pick one, submit, verify reply JSON lands
   in the watched dir with correct schema.
8. Survey rendering: 4×3 variant (mock up an event with
   multiple questions). Submit, verify reply.
9. "Check my comments first" standing option appears on
   every survey.
10. Scope-grant selector defaults to one-shot; can be
    upgraded per survey.
11. Stack vs tray user preference: toggle via prefs,
    verify both shapes work.
12. Terminal-tab status bullet appears when watcher is
    attached; blinks on new bubbles while prompt is
    hidden; clears on prompt re-open.

### Carry-over verdicts

13. Re-confirm `fullstack-11` (fs-move UX wedge) and
    `fullstack-12` (Cmd+T rebind) on current main.
    Quick smoke, not full sweep.

## How to start

* Test drive `/tmp/chan-webtest-a-1/` and port 8801 still
  yours. Rebuild + bounce server after each commit.
* For synthetic events:
  ```bash
  mkdir -p /tmp/chan-test-events
  cat > /tmp/test-event.json <<'EOF'
  {"id":"t1","type":"survey","from":"@@TestAgent",
   "to":"@@Architect","topic":"sanity",
   "questions":[{"header":"OK?","text":"Test?",
   "options":[{"key":"1","label":"yes"},
              {"key":"2","label":"no"}]}],
   "standing_options":[{"key":"C","label":"Check my comments first"}],
   "scope":"one-shot"}
  EOF
  mv /tmp/test-event.json /tmp/chan-test-events/event-1.md
  ```
* Permission scope carried over.

## Hand-off

Ping after each cluster via
`alex/event-webtest-a-architect.md`.
