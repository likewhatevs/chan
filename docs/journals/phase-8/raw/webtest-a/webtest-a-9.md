# webtest-a-9 — -a-63 chip count visual + -a-56 retest (Cmd+P / depth slider)

Owner: @@WebtestA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Two-part walk:

1. **`-a-63` chip count visual**: verify contact chip
   displays ~48 (down from 1982) on chan-source seed
   drive.
2. **`-a-56` retest**: the prior `webtest-a-8` build
   incident blocked Cmd+P + depth slider checks;
   retest them now that `-a-56` is in HEAD as
   `9f0ac44`.

## Reference

* `-a-63` task body + commit `19d3d4f`.
* `-a-56` task body + commit `9f0ac44`.
* `webtest-a-8` build incident: Chrome MCP couldn't
  resize FB column (file-MOVE triggered); `-a-56`
  in-flight code blocked the initial build.

## Acceptance

### -a-63 chip count

1. **Contact chip ~48**: drive-scope graph; contact
   chip displays ~48 (or close — minor variance OK
   due to handle-name dedup variants).
2. **Other chips audited**: tag / language / folder
   chips show node-count not edge-count semantics.
   Folder count specifically should NOT double-count
   (pre-`-a-63` was edge + node tally; now node-only).

### -a-56 Cmd+P 3-state contract

3. **Cmd+P on terminal tab, prompt NOT showing** →
   prompt opens on current terminal.
4. **Cmd+P on terminal tab, prompt IS showing** →
   prompt HIDES (toggle off).
5. **Cmd+P on non-terminal tab** → spawns terminal +
   opens prompt.

### -a-56 depth slider shallow-scope cue

6. **Slider at shallow scope**: open graph scoped to
   a file with only depth-1 reach; confirm visual
   cue indicates max=1 already reveals everything.

### Walkthrough audit trail

Append to [`webtest-a-1.md`](webtest-a-1.md):
`## 2026-05-22 — fullstack-a-63 chip count + fullstack-a-56 retest`.
Verdicts + screenshots + tear-down.

## How to start

1. Confirm `19d3d4f` + `9f0ac44` in HEAD.
2. Rebuild chan; spin up test server + chan-source seed.
3. Walk -a-63 checks (1-2): graph + chip inspection.
4. Walk -a-56 checks (3-6): rich prompt + slider.
5. Append verdict; fire poke; tear down.

## Coordination

* @@WebtestA lane.
* Light walk; ~20 min.

## Numbering

This is `-9`.

## Out of scope

* `-a-62` resize retest (Chrome MCP tooling blocked; deferred).
* `-a-58` parent-edge (already 3/4 HOLD).
* `-a-59` / `-a-60` (separate future walks).
