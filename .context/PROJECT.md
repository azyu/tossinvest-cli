---
created: 2026-06-18
updated: 2026-06-18
---
# tossinvest-cli

Toss Securities Open API wrapper CLI. Rust workspace planned with a small CLI crate and a reusable core crate.

## Current State

Phase 1 read-only CLI is implemented and final-reviewed. Phase 2 typed wrapper/library core is now active.

- Spec: `docs/superpowers/specs/2026-06-18-tossinvest-cli-design.md`
- Phase 1 plan: `docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md`
- Reference CLI reviewed: `/Volumes/EXTSSD/code/personal/kis-cli`
- Toss API source of truth: `https://openapi.tossinvest.com/openapi-docs/latest/openapi.json`

## Delivery Order

1. Read-only investment terminal.
2. Typed wrapper/library core.
3. Real order-capable CLI.
## Planned Tech Stack

- Rust edition 2024
- `toss-core`: config, auth, transport, API client, read-only wrappers
- `toss-cli`: clap parser, command dispatch, text/json output
- `reqwest` with rustls, `tokio`, `serde`, `serde_json`, `serde_yaml`, `thiserror`, `anyhow`

## Key Decisions

- Use Approach B: OpenAPI-based core plus hand-designed CLI UX.
- Keep `kis-cli` style where useful: 2-crate split, config/env overrides, JSON envelope, text output, smoke tests.
- Do not copy KIS-specific TR-ID, hashkey, virtual/real environment, or WebSocket concepts.
- Do not expose mutating order commands until dry-run, confirmation, idempotency, and tests are in place.
