# frontend-11: graph filter chip counter for the merged graph

Owner: @@Frontend
Status: PARKED to phase 6.1 per [architect-4](./architect-4.md)
(cosmetic mismatch; underlying graph data is correct).

## Goal

Fix the graph overlay's filter chip counts when consuming the
merged `/api/graph` payload from
[backsystacean-9](./backsystacean-9.md). Chip counts must match
the underlying node count by kind.

## Symptom

From @@WebtestA browser pass in [webtest-1](./webtest-1.md)
(filed as **OBS-WT6-WTA-9**):

* Chip `folder 19` while underlying graph payload contains 4
  directory nodes.
* Chip `contact 7` while underlying graph payload contains 3
  contact files + 2 mentions = 5 max.
* Backend data is correct; the chip counter overcounts.
* Likely cause: the counter is summing edge endpoints or
  inferring kinds from a place that double-counts after the
  fs / language layers merged into `/api/graph`.

## Source

* Filter chip row in `web/src/components/GraphPanel.svelte`
  (the row containing `link / tag / contact / language /
  media / folder` chips).
* Node iteration that feeds the counts; likely a `reduce` /
  group-by-kind loop. May still reference the old semantic-only
  payload assumptions.

## Scope

* Count chips by **distinct node id**, grouped by `kind`. No
  inference from edges.
* The new merged response carries `directory` / `file` /
  `media` / `language` kinds; ensure the counter respects them.
* Once [frontend-5](./frontend-5.md)'s codemod ships, the
  `folder` chip label becomes `directory`; the counter logic
  should not depend on the label, only on the node `kind`.

## Acceptance criteria

* Chip counts equal the node count for that kind on every
  scope (drive / directory / file).
* Live verification on the seeded test drive: counts match
  manual node enumeration.

## Tests

* Vitest covering the counter logic on a fixture payload
  representative of the merged response.
* `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build` green.

## Relevant links

* Observation: [webtest-1](./webtest-1.md) OBS-WT6-WTA-9.
* Backend producer: [backsystacean-9](./backsystacean-9.md).
* Frontend chip wiring: [frontend-4](./frontend-4.md).

## Progress notes

(populated as work lands)

## Completion notes

(populated when ready for commit)
