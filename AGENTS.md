# AGENTS.md

## Project

`tossinvest-cli` is a Rust command-line wrapper for the Toss Securities Open API.

The approved delivery order is:

1. Read-only investment terminal.
2. Typed wrapper/library core.
3. Real order-capable CLI.

Do not expose mutating order commands until dry-run, confirmation, idempotency, and order error handling are implemented and tested.

## Required Context Intake

Before starting any task, read these files in order:

1. `.context/PROJECT.md` — current project summary and links to approved design/plan.
2. `.context/STEERING.md` — active priorities, constraints, and decision log.
3. `.context/TASKS.md` — current status board.

The `.context` directory is the lightweight coordination layer for future sessions and agents. Keep it current when task status changes, but do not copy full specs or implementation plans into it.

Durable documents live here:

- Design spec: `docs/superpowers/specs/2026-06-18-tossinvest-cli-design.md`
- Phase 1 implementation plan: `docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md`

## Source of Truth

Use the Toss OpenAPI JSON as the API source of truth:

- `https://openapi.tossinvest.com/openapi-docs/latest/openapi.json`

Human-readable references:

- `https://openapi.tossinvest.com/openapi-docs/overview.md`
- `https://openapi.tossinvest.com/openapi-docs/latest/api-reference/README.md`

## Architecture Direction

Use a two-crate Rust workspace:

```text
rust/
├── toss-core/   # config, auth, HTTP transport, API client, endpoint wrappers
└── toss-cli/    # clap parser, command dispatch, text/json output, binary behavior
```

Follow the `kis-cli` reference for useful CLI discipline only:

- 2-crate split.
- config file plus environment overrides.
- stable JSON success/error envelopes.
- text output for humans.
- binary smoke tests.

Do not copy KIS-specific concepts:

- TR-ID routing.
- hashkey generation.
- real/virtual environment split.
- WebSocket approval/streaming behavior.

## Implementation Rules

- Prefer boring, explicit Rust over abstractions that are not yet needed.
- Keep endpoint wrappers thin: build query/body, call client, parse `result`.
- Make network behavior testable through a mock transport; tests must not require real Toss credentials.
- Keep prices, quantities, and money values as strings or `serde_json::Value` in Phase 1. Do not use floating-point types for financial values.
- Never print `client_secret` or access tokens in normal command output.
- JSON mode must print both success and error envelopes to stdout.
- Text mode prints human output to stdout and errors to stderr.
- Do not introduce order create/modify/cancel calls during Phase 1.

## Verification

Before claiming completion, run the focused command for the changed area and record the observed result.

Definition of Done for feature or behavior changes:

- Run `cargo fmt --manifest-path rust/Cargo.toml --all`.
- Run `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings`.
- Run `cargo test --manifest-path rust/Cargo.toml`.
- Run `cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss`.
- When a CLI path can be exercised without live mutation, run a smoke or dry-run command and record the observed result.

Phase 1 final verification requires:

```bash
cargo fmt --manifest-path rust/Cargo.toml
cargo test --manifest-path rust/Cargo.toml
cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss
```

Also run the config smoke command from the Phase 1 plan once the CLI exists.

## Updating `.context`

Update `.context/TASKS.md` when a Phase 1 task starts or completes.

Update `.context/STEERING.md` only for durable decisions that should affect future sessions.

Update `.context/PROJECT.md` when the project state changes materially, such as Phase 1 completion or a new active phase.
