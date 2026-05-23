## Summary

A short description of what this PR changes and why.

## Related issue / discussion

Link to a tracking issue or design discussion if applicable. PRs that ship without prior discussion are fine for small fixes; non-trivial features should reference an issue.

## Pre-push gate

The repo ships a pre-push hook (`./scripts/install-hooks` to install) that runs the same checks CI enforces. Tick what's been verified locally:

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `cargo build --no-default-features`
- [ ] `cd web && npm run check`
- [ ] `cd web && npm test -- --run`
- [ ] `cd web && npm run build`

## Notes for reviewers

Anything reviewers should pay extra attention to: subtle invariants, intentional tradeoffs, follow-up work parked for a later PR.
