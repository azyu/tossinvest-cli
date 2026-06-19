# Steering

> Project direction, priorities, and constraints.
> Read before starting any implementation task.

## Current Priority

Phase 3 order-capable CLI is implemented and final verification passed.
## Execution Mode

File-based coordination.

- Durable design lives in `docs/superpowers/specs/2026-06-18-tossinvest-cli-design.md`.
- Step-by-step implementation lives in `docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md`.
- This `.context` directory tracks current state and steering only; do not duplicate the full plan here.

## Constraints

- OpenAPI JSON is the source of truth: `https://openapi.tossinvest.com/openapi-docs/latest/openapi.json`.
- Follow the delivery order: read-only CLI → typed wrapper/library core → order-capable CLI.
- Mutating order commands require `--dry-run` for non-mutating smoke and `--confirm` for live execution; no documented sandbox/staging is assumed.
- Do not copy KIS-specific TR-ID, hashkey, virtual/real environment, or WebSocket concepts.
- Keep endpoint wrappers thin: build query/body, call client, parse `result`.
- Do not use floating-point types for financial values; keep values as strings or `serde_json::Value` in Phase 1.
- Never print `client_secret` or access tokens in normal command output.
- JSON mode must use stable success/error envelopes for automation.
- Network-dependent behavior must be testable through a mock transport.

## CLI UX Direction

Global flags:

- `--config <path>`
- `--account <accountSeq>`
- `--output text|json`
- `--json`
- `--quiet`

Phase 1 commands:

- `toss config`
- `toss auth token`
- `toss price <symbol>`
- `toss quote orderbook|trades|limits <symbol>`
- `toss chart candles <symbol>`
- `toss stock get|warnings|search`
- `toss market exchange-rate|calendar`
- `toss account list|use`
- `toss holdings`

## Decisions Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-18 | Use `kis-cli` as a style reference, not as code to port directly | Toss API lacks KIS TR-ID/hashkey/virtual/WebSocket concepts and has a canonical OpenAPI spec |
| 2026-06-18 | Choose Approach B: OpenAPI-based core plus hand-designed CLI UX | Balances correctness against human-friendly commands |
| 2026-06-18 | Implement in order: read-only CLI → typed core → order CLI | Avoids exposing production order calls before safety design is implemented |
| 2026-06-18 | Add `.context` as a lightweight state tracker | Keeps future sessions oriented without duplicating spec and plan documents |

## Notes

- Public Toss docs inspected so far show a single production server: `https://openapi.tossinvest.com`.
- Mutating order commands require `--dry-run` for non-mutating smoke and `--confirm` for live execution.
- No documented sandbox/staging support is assumed; live order traffic is treated as production.
- JWKS is mentioned in high-level orientation but was not confirmed in the API reference index during design review.
