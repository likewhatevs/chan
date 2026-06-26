---
name: Feature request
about: Suggest a change or addition to chan
title: ''
labels: enhancement
assignees: ''
---

## Problem / motivation

What user-facing problem are you trying to solve? Concrete use case is more valuable than abstract framing.

## Proposed solution (optional)

A short description of the change you have in mind. Sketches, mockups, or pseudo-code welcome but optional.

## Alternatives considered

If you tried other approaches or thought through alternatives, mention them so reviewers can see the tradeoffs you weighed.

## Fits the chan shape?

chan keeps a tight feature surface. Quick sanity check before opening:

* Does it fit a single-binary, no-runtime-deps AI-native IDE?
* Does it preserve the chan-drive boundary (no direct filesystem ops outside the drive contract)?
* Does it stay local-first by default (no required network calls)?

If unsure, that's fine. Open the issue and we'll discuss.
