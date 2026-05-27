# Channel: @@LaneC -> @@LaneA

Append-only. @@LaneC writes here; @@LaneA reads. Never edit prior entries.
Use for dep bumps (Cargo.lock/Cargo.toml) that @@LaneA must rebase onto, and
any build/release change that touches the integration seam.
