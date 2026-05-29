# Addendum-2 review (round-2 prep)

@@Lead -> @@Alex. Answer inline after each `@@Alex:` marker (editor-friendly).
Nothing here is dispatched yet - these scope the addendum-2 work; @@Lead routes
it to the lanes once you rule. Your `request.md` is untouched (you said do not
edit it); this is a sibling review file.

## Proposed routing (confirm or redirect)

| Item                                   | Proposed lane | Why                                            |
|----------------------------------------|---------------|------------------------------------------------|
| FB tab vs docked FB independent expand | @@LaneA       | owns FileTree/FB state; just did the A4 dock-browserState decouple |
| Editor reload + cursor jump to 1:1     | @@LaneC       | reload cause = the self-write follow-up already authorized; cursor-restore is its editor sibling |
| Terminal font after sleep              | @@LaneC       | one recovery pass with the held Bug 1 (resize clue gives the fix) |
| Drag-drop image / paragraph move       | @@LaneC       | editor command work                            |
| Shortcuts policy                       | @@LaneE (new) | big, cross-platform, touches the keymap registry AND the desktop key-bridge |

NOTE: @@LaneD is the CI + release lane now, so a "new lane" for shortcuts would
be @@LaneE (not D).

@@Alex (routing overrides, if any):


## Questions

### 1. FB independent expansion -> @@LaneA?
The "expanding a dir in the FB tab also expands it in the docked FB" bug is
FileTree/FileBrowserSurface state - @@LaneA's surface (it just shipped the dock
getting its own browserState in A4). Route it there rather than @@LaneC?

@@Alex: yes, I am on v0.15.5 so the bug may be gone in the current codebase from what you tell me


### 2. Shortcuts -> dedicated @@LaneE, or load onto @@LaneC?
The shortcuts policy is sizable and cross-platform (web + desktop-native macOS/
Linux) and spans the keymap/chord registry AND the desktop key-bridge in
`serve.rs`. @@LaneC's round-2 plate is already full (self-write follow-up +
cursor-restore + terminal recovery + drag-drop). I lean toward a dedicated
@@LaneE for shortcuts. Your call.

@@Alex: laneE, give me the ONE LINE with bootstrap pointing to a doc with the rest, prompt and process etc


### 3. Editor reload + cursor jump - two facets?
I read this as two fixes: (a) STOP the spurious reload (the self-write race at a
site Bug 3 did not cover - the follow-up slice already authorized for @@LaneC);
(b) PRESERVE cursor position on any reload (so even a legit external edit does not
snap to line 1, col 1). Confirm you want both - (b) is worth doing regardless,
since real external reloads should not lose the cursor either.

@@Alex: obviously it's A to me; no editor should be reloading while i am typing, and no updating of the current document either.. popping a warning at the top because the underlying fs changed the content is desired and fine, and so is making the file as 'locked' if the underlying fs does e.g. chmod -w file.md, but other than that, no reason to stop my flow while i type in the editor; and i type cmd+r to reload the whole window, i want my cursor in the exact same spot and cursor focus as well


### 4. Drag-drop / paragraph move - scope now?
Confirm scope = the easy single-row case (`text ![](..) text`, move the whole
row) + the bullet-list case now, and DEFER the ambiguous prose-paragraph
detection (the 80-col / period-or-not "what is a paragraph" problem) to later.

@@Alex: yes the easy case, no paragraph complex cases


### 5. Web vs desktop chord split - rationale?
Web uses `alt+[/]` (+ `alt+shift+[/]`) while desktop uses `cmd+[/]` and
`cmd+1..9`. I read that as deliberate: on web those `cmd` chords are browser-
reserved (tab switch, back/forward), so web falls back to `alt`; desktop (Tauri,
no browser chrome) can use `cmd`. Confirm that is the intent - and that on web we
should preventDefault the ones the browser would otherwise eat (e.g. `cmd+s`).

@@Alex: intentional to use alt on web, yes.. alt+shift+[/] for tabs, alt+[/] for panes


### 6. `cmd+w` / `ctrl+d` (line 30) - what does it close, and the Linux collision?
The line is ambiguous: does `cmd+w` / `ctrl+d` close a TAB or a PANE? (You also
say `cmd+w` on an EMPTY pane closes the window.) And on Linux, `ctrl+w` / `ctrl+d`
collide with terminal readline (delete-word / EOF). How do you want that resolved
- context-aware (terminal keeps readline; close applies elsewhere / on empty
panes)?

@@Alex: closes the current tab, if no more tabs closes the pane (already like this today), and if no panes then closes the window; brings focus back to the native-desktop list of workspaces


### 7. `ctrl+a` on Linux - context split?
Confirm: on Linux, `ctrl+a` = select-all in the EDITOR but stays readline
"beginning of line" in the TERMINAL (context-dependent). On macOS, `ctrl+a` stays
beginning-of-line everywhere and `cmd+a` is select-all.

@@Alex: correct


### 8. `cmd+i` infographics tab - net-new surface?
You flagged "new?" - this reads as a brand-new tab TYPE ("infographics") that does
not exist yet, not just a keybinding. That is a separate feature, out of scope for
a shortcuts pass. Confirm it is a separate feature (and whether it is even in play
this phase), so the shortcuts lane does not block on it.

@@Alex: we already have this tab, it's the infographics.. i flagged new because i thought the shortcut is new.. tho iirc we already have "cmd+. i" for that? if not, we should too


### 9. `cmd+f` find-in-document / `cmd+g` next - exists or net-new?
Does the editor have in-document find today, or is `cmd+f` / `cmd+g` /
`cmd+shift+g` net-new CodeMirror search work bundled into the shortcuts lane?
(This is distinct from `cmd+s` = drive-wide search.) Affects the size of that
lane.

@@Alex: we already have cmd+f in tauri today, and it is disabled for web cases; we just need to validate / verify and confirm the triad cmd+f cmd+f cmd+shift+g like the browser behaviour that users are already familiar with; i think "esc" on the find in page should close the find in page tho iirc cmd+g and cmd+shift+g should remain working, so long they dont automatically interfere with the scrolling of the window except when the user hits the shortcut

