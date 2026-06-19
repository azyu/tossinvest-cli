---
created: 2026-06-18
updated: 2026-06-19
---
# Tasks

> Read this file after `.context/PROJECT.md` and `.context/STEERING.md`.
> Keep this as a status board. Do not duplicate the full implementation plan here.

## Active Work Tree

### [x] Phase 1: Read-only Tossinvest CLI

Goal: implement the approved Phase 1 plan for a read-only Toss Securities Open API CLI.

Source plan: `docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md`

| Status | Task | Context / Verification |
|--------|------|------------------------|
| [x] | Workspace and config foundation | Complete. Verified with `cargo test --manifest-path rust/Cargo.toml -p toss-core config::tests` → 5 passed. Commits `51751c9`, `553bee0`. |
| [x] | Transport and auth token manager | Complete. Verified with `cargo test --manifest-path rust/Cargo.toml -p toss-core auth::tests` → 5 passed. Commits `66c5239`, `a37bb14`, `927bab9`. |
| [x] | Authenticated client and endpoint wrappers | Complete. Verified with `cargo test --manifest-path rust/Cargo.toml -p toss-core` → 17 passed. Commit `ca0de61`. |
| [x] | CLI parser and output runtime | Complete. Verified with `cargo test --manifest-path rust/Cargo.toml -p toss-cli` → 4 passed. Commit `18cf072`. |
| [x] | Wire read-only commands | Complete. Verified with `cargo test --manifest-path rust/Cargo.toml` → 23 passed. Commit `a2e5097`. |
| [x] | Documentation and final verification | README added; verified with `cargo fmt --all --manifest-path rust/Cargo.toml`, `cargo test --manifest-path rust/Cargo.toml` → 35 passed, `cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss`, and `cargo run --manifest-path rust/Cargo.toml -p toss-cli --bin toss -- --config <temp-config> --json config`. |

## Blocked / Deferred

| Status | Task | Context |
|--------|------|---------|
| [ ] | Phase 2 typed wrapper/library core | In progress. Task 1 and Task 2 are complete; tasks 3+ remain. |
| [ ] | Phase 3 order-capable CLI | Deferred until typed core, dry-run, confirmation, idempotency, and order error tests are designed and implemented. |

## Completed

| Date | Item | Evidence |
|------|------|----------|
| 2026-06-18 | Design spec approved | `docs/superpowers/specs/2026-06-18-tossinvest-cli-design.md`, commit `06ef6db` |
| 2026-06-18 | Phase 1 implementation plan written | `docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md`, commit `b9005aa` |
| 2026-06-18 | `.context` structure adopted | `PROJECT.md`, `STEERING.md`, `TASKS.md` |
| 2026-06-19 | Documentation and final verification | `README.md`, `cargo fmt --all --manifest-path rust/Cargo.toml`, `cargo test --manifest-path rust/Cargo.toml`, `cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss`, and `cargo run --manifest-path rust/Cargo.toml -p toss-cli --bin toss -- --config <temp-config> --json config` |
| 2026-06-19 | Phase 2 Task 2 typed market data wrappers | Complete. Verified with `cargo test --manifest-path rust/Cargo.toml -p toss-core market_data::tests` → 2 passed, `cargo test --manifest-path rust/Cargo.toml -p toss-core` → 25 passed. Commit `775e0c4`. |
