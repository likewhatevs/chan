# @@Architect task 3: rewrite the three chan-term commit messages

Owner: @@Architect
Status: TODO — drafts below; execution waits for Alex's nod.
Coordinates with: [architect-2](./architect-2.md) (commit coordination
for the phase).

## Goal

The three unpushed local commits on `main` (the chan-term terminal
work) carry messages in a noticeably different style from the rest
of the repo. Alex asked for a rewrite before this phase's stack
lands on `origin/main`.

## Commits to rewrite

```
0f4614e web: add terminal overlay
980fc3e web: move terminal into workspace tabs
963bade web: add terminal tab controls
```

Safe to rewrite — these are local-only on this checkout and were
never on `origin`.

## Style we're matching

Sampled from `06017f4` ("web: file inspector parity ...") and
`d6afc3f` ("web: drive root reads as a distinct node ..."):

* Subject: lowercase scope, imperative, ~50-70 chars.
* Body wrapped to ~68 columns.
* Opening paragraph names the user-facing problem and frames why
  this change. Not "Add X..."; instead "X was missing because... so
  this change..." or equivalent.
* Body sectioned by surface name (`web`, `chan-server`, `desktop`,
  `tests`) with bullets under each header, no leading "- ".
  Actually inspect the canonical commits: they use a single dash
  bullet for each line under a header. Match that exactly.
* `Verification:` trailer with concrete commands and what was
  exercised, wrapped to the same 68 columns.

## Drafts

Drafted in the form they'll land. Read each, push back on anything,
then I'll run the rebase.

### 0f4614e → web: add terminal overlay

```
web: add terminal overlay

The editor surface had no way to drop into a shell inside the
current drive. Adding xterm.js wired into a new chan-server
PTY WebSocket so the shell launches under the drive root with
the existing overlay stack, URL hash routing, and empty-pane
navigation.

web
- New OverlayShell mount for the terminal, registered in the
  overlay stack and hash-state so reload restores the open
  state.
- Wired into the command bridge and empty-pane navigation so
  shortcuts land in the same surface other overlays use.
- xterm.js handles fit, search, scrollback copy, restart, and
  ANSI color output.

chan-server
- New /api/terminal/ws route using portable-pty. Shell cwd is
  the drive root; resize and input arrive as JSON control
  frames; output is binary; shutdown closes the child
  cleanly.
- Public-tunnel mode refuses the upgrade so the gateway never
  forwards an interactive shell.
- Terminal env is normalised for color-capable shells.

tests
- Conditional real-PTY validation exercising tty, stty, tput,
  sh read/write, and less when those programs are present in
  PATH.

Verification: cargo check -p chan-server; the focused PTY
test; npm run check; npm run build; cargo build -p chan.
```

### 980fc3e → web: move terminal into workspace tabs

```
web: move terminal into workspace tabs

The terminal-as-overlay model lost state every time the user
switched scopes or panes because the overlay unmounted on
close. Promoting the terminal to a first-class workspace tab
so it stays mounted while inactive, participates in the same
tab model file editors use, and is owned by the pane the user
opened it in.

web
- Replaces the OverlayShell terminal mount with a TerminalTab
  component registered as a tab kind in the workspace state.
  The tab stays mounted across pane focus changes.
- Pane state tracks the terminal tab alongside file tabs so
  reordering, focus, and close behave the same across kinds.

chan-server
- PTY sessions still cwd into the drive root, but keep HOME
  and the user's login shell default so shell profiles
  (prompt, color, aliases) load the same way they would in
  the host terminal.

Verification: npm run check; npm run build; cargo check -p
chan-server; cargo test -p chan-server
routes::terminal::tests::conditional_pty_programs_validate_real_terminal
-- --nocapture; live WebSocket PTY cwd/HOME smoke against
http://127.0.0.1:8797/.
```

### 963bade → web: add terminal tab controls

```
web: add terminal tab controls

After the terminal moved into workspace tabs, three usability
gaps surfaced: tabs were unnamed, broadcasting input across
sessions had no app-level control surface, and pane navigation
chords were missing. Closing all three together.

web
- Terminal-tab title menu for renaming the tab and configuring
  the set of broadcast-input target sessions. Right-clicking
  inside the terminal viewport now stays with xterm and the
  browser; the app menu opens from the tab title only, which
  matches user expectation for terminal apps.
- Mod+Shift+I toggles broadcast-input on the current tab
  without clearing the remembered target set, so the user can
  flip broadcasting on and off without redoing the selection.
- Tab metadata (name, broadcast targets) persists in the
  workspace state so it survives reload alongside the other
  tab descriptors.
- New Mod+[ and Mod+] chords navigate between panes; the
  desktop key bridge forwards the same chords, and the
  chan serve help shortcut table now lists them.

chan-server
- The terminal tab name flows through on the /api/terminal/ws
  connect and is exported as CHAN_TAB_NAME in the PTY
  environment, so shells and any spawned tools can read which
  tab they are in.

Verification: npm run check; npm run build; cargo check -p
chan-server; cargo check -p chan; cargo check -p chan-desktop;
cargo test -p chan-server
routes::terminal::tests::conditional_pty_programs_validate_real_terminal
-- --nocapture; live WebSocket PTY cwd/HOME/CHAN_TAB_NAME
smoke against http://127.0.0.1:8797/.
```

## Execution plan (non-interactive)

The harness disallows `git rebase -i`. Non-interactive recipe that
rewords each commit by subject match (so it survives rebased
hashes):

```
# 1. stage the three message bodies
cat > /tmp/msg-0f4614e.txt <<'EOF'
<paste 0f4614e draft above>
EOF
cat > /tmp/msg-980fc3e.txt <<'EOF'
<paste 980fc3e draft above>
EOF
cat > /tmp/msg-963bade.txt <<'EOF'
<paste 963bade draft above>
EOF

# 2. rebase, picking the right body per commit by current subject
git rebase HEAD~3 --exec '
  case "$(git log -1 --format=%s)" in
    "web: add terminal overlay")
        git commit --amend -F /tmp/msg-0f4614e.txt ;;
    "web: move terminal into workspace tabs")
        git commit --amend -F /tmp/msg-980fc3e.txt ;;
    "web: add terminal tab controls")
        git commit --amend -F /tmp/msg-963bade.txt ;;
    *) echo "unexpected subject: $(git log -1 --format=%s)"; false ;;
  esac
'

# 3. sanity-check
git log --format='%H %s%n%n%b' -3
```

The exec block runs after each replayed pick so it amends *that*
commit, regardless of the rebased SHA. The case-statement matches
on the current subject line, which all three carry verbatim
through the rebase.

Roll-back if anything goes sideways: `git reflog` shows the
pre-rebase HEAD; `git reset --hard <reflog-sha>` restores it.

## Acceptance criteria

* Subject lines unchanged.
* Bodies match the drafts above (after any push-back from Alex).
* `git log` shows the same three commits in order, same trees, new
  messages.
* `git diff origin/main..HEAD` content is byte-identical to the
  pre-rebase diff (no code changed).
* Pre-push gate still green from the systacean-1 / systacean-4
  baselines (the gate is content-only; commit messages don't move
  it).

## Sequencing

Run this rebase **after** wave-2 finishes and the cleanup commits
are queued for landing in [architect-2](./architect-2.md), but
**before** the final push. That way the per-lane commit groupings
in architect-2 can sequence "rewrite terminal trio" between the
existing terminal commits and the cleanup commits without an extra
push to `origin`.

## Progress

* 2026-05-17 @@Architect drafted the three replacement bodies
  above. Holding for Alex sanity-check; will run the rebase on the
  green-light.

## Completion notes

(populated after the rebase; include the new `git log` summary)
