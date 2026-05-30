# Phase 15 round 1
Multiple enhancement requests, and some fixes.

Author: @@Alex
2026-05-30 

**Agents:** do not read, do not use, do not edit.

## Chan Shell
We are going to introduce the `cs` command line to implement the functionality for chan shell: a way to control chan's UI via the terminal.
1. We are NOT going to introduce a new binary
2. We are going to implement `cs` in the same `chan` binary, and we will hit this code path through a symlink.
3. For now, we will not auto-create the symlink and will use the symlink creation as a gate for testing

## Terminal
We are going to introduce broadcast groups, so that we can enable Hybrid Terminal instances to join and part groups. In today's implementation, the broadcast funcionality is on/off for all existing terminals. With this new feature, the existing funcionality remains.
