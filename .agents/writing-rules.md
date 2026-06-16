# Writing Rules

- **No em dashes** in comments or documentation.
- **Tables**: pure ASCII, target 80 columns.
- **Factual**: no marketing language. Include analysis with benchmarks; explain whether numbers meet expectations.
- **Comments**: explain WHY, not WHAT.
- **Snapshot, not changelog**: comments and docs describe the code as it IS, in the present tense (the WHY behind its current shape). Never narrate change over time or cite plan / round / phase numbers or task ids ("§2b", "until §2a splits this out", "previously X, now Y", "TODO round 2"). `CHANGELOG.md` is the only place change history belongs, under its own discipline. The `dev/` coordination tree (plans, journals, task files) is exempt: phase and round references are its purpose. A real runtime phase ("embed phase", "BM25 phase") is fine; this bars PLAN-phase references.
