---
created: 2026-06-18
updated: 2026-06-18
---
# Tasks

> Read this file after `.context/PROJECT.md` and `.context/STEERING.md`.
> Keep this as a status board. Do not duplicate the full implementation plan here.

## Active Work Tree

### [ ] Phase 1: Read-only Tossinvest CLI

Goal: implement the approved Phase 1 plan for a read-only Toss Securities Open API CLI.

Source plan: `docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md`

| Status | Task | Context / Verification |
|--------|------|------------------------|
| [x] | Workspace and config foundation | Complete. Verified with `cargo test --manifest-path rust/Cargo.toml -p toss-core config::tests` → 5 passed. Commits `51751c9`, `553bee0`. |
| [x] | Transport and auth token manager | Complete. Verified with `cargo test --manifest-path rust/Cargo.toml -p toss-core auth::tests` → 5 passed. Commits `66c5239`, `a37bb14`, `927bab9`. |
| [ ] | Authenticated client and endpoint wrappers | Add `TossClient` and read-only endpoint wrappers. Verify with `cargo test --manifest-path rust/Cargo.toml -p toss-core`. |
| [ ] | CLI parser and output runtime | Add `toss-cli`, clap parser, JSON envelopes, config smoke test. Verify with `cargo test --manifest-path rust/Cargo.toml -p toss-cli`. |
| [ ] | Wire read-only commands | Dispatch Phase 1 commands to `toss-core`. Verify with `cargo test --manifest-path rust/Cargo.toml`. |
| [ ] | Documentation and final verification | Add README, run `cargo fmt`, `cargo test`, `cargo build`, and config smoke command from the plan. |

## Blocked / Deferred

| Status | Task | Context |
|--------|------|---------|
| [ ] | Phase 2 typed wrapper/library core | Deferred until Phase 1 read-only CLI works. |
| [ ] | Phase 3 order-capable CLI | Deferred until typed core, dry-run, confirmation, idempotency, and order error tests are designed and implemented. |

## Completed

| Date | Item | Evidence |
|------|------|----------|
| 2026-06-18 | Design spec approved | `docs/superpowers/specs/2026-06-18-tossinvest-cli-design.md`, commit `06ef6db` |
| 2026-06-18 | Phase 1 implementation plan written | `docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md`, commit `b9005aa` |
| 2026-06-18 | `.context` structure adopted | `PROJECT.md`, `STEERING.md`, `TASKS.md` |
