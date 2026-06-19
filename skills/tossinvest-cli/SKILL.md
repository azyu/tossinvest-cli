---
name: tossinvest-cli
description: This skill should be used when the user asks to "work on tossinvest-cli", "use the Toss Securities CLI", "verify Toss API credentials", "test Toss Open API", "add Toss order commands", "run toss order dry-run", or mentions the `toss` binary, Toss Securities Open API credentials, order safety, accountSeq, dry-run orders, or the `/Volumes/EXTSSD/code/personal/tossinvest-cli` repository.
version: 0.1.0
---

# Tossinvest CLI Skill

Use this skill to work safely with the `tossinvest-cli` Rust workspace and the installed `toss` binary. Treat Toss Securities Open API as a production-capable brokerage API. Prefer read-only checks and dry-run order checks unless the user explicitly requests a live order and all safety preconditions are satisfied.

## Core Context

Start every repository task by reading these files in order:

1. `.context/PROJECT.md` — current project state and approved phase links.
2. `.context/STEERING.md` — durable constraints and decision log.
3. `.context/TASKS.md` — current status board.
4. `AGENTS.md` — repository-wide instructions.

Durable implementation documents:

- Design spec: `docs/superpowers/specs/2026-06-18-tossinvest-cli-design.md`
- Phase 1 plan: `docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md`
- Phase 2 plan: `docs/superpowers/plans/2026-06-19-tossinvest-cli-phase2.md`
- Phase 3 plan: `docs/superpowers/plans/2026-06-19-tossinvest-cli-phase3.md`

## Repository Shape

Use the Rust workspace under `rust/`:

```text
rust/
├── toss-core/   # config, auth, transport, typed API wrappers, order wrappers
└── toss-cli/    # clap parser, runtime dispatch, text/json envelopes, binary
```

Key commands:

```bash
cargo fmt --all --manifest-path rust/Cargo.toml
cargo test --manifest-path rust/Cargo.toml
cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss
cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss --release
```

Use `cargo fmt --all --manifest-path rust/Cargo.toml`; the workspace manifest has no direct targets, so plain `cargo fmt --manifest-path rust/Cargo.toml` is not the reliable command here.

## Credential Rules

Keep credentials outside the repository. Never commit real `client_id`, `client_secret`, access tokens, account numbers from private contexts, token cache files, or local config files.

Supported local config path:

```text
~/.config/tossinvest/config.yaml
```

Safe file setup:

```bash
mkdir -p ~/.config/tossinvest
chmod 700 ~/.config/tossinvest
$EDITOR ~/.config/tossinvest/config.yaml
chmod 600 ~/.config/tossinvest/config.yaml
```

Expected config shape:

```yaml
client_id: "issued-client-id"
client_secret: "issued-client-secret"
```

Persist an account only when account-bound commands are needed:

```bash
toss account list
toss account use 1
```

Session-only environment alternative:

```bash
export TOSSINVEST_CLIENT_ID="issued-client-id"
export TOSSINVEST_CLIENT_SECRET="issued-client-secret"
export TOSSINVEST_ACCOUNT_SEQ="1"
```

Token cache path:

```text
~/.tossinvest/token.json
```

Do not read, print, paste, or summarize token cache contents unless diagnosing file permissions. Never include token values in reports.

Treat account, holdings, orders, buying-power, and order-history output as private financial data. Summarize status and payload shape unless the user explicitly asks for exact values.

## Safe Verification Workflow

Use this sequence when asked to verify credentials or installation. Stop at the first failure and report the exact command and observed output shape without exposing secrets or private financial details.

```bash
toss --json config
toss --json auth token
toss --json account list
toss --json holdings
toss --json order buying-power --currency USD
toss --json order buy --symbol AAPL --qty 1 --type limit --price 1 --dry-run
```

Expected safety behavior:

- `config` masks `client_id` and never prints `client_secret`.
- `auth token` returns `token_check: ok` and never prints the token.
- `account list`, `holdings`, and `order buying-power` are read-only.
- `order buy ... --dry-run` prints method/path/body/account-header presence and never sends a live order.

Run this negative live-order gate test only when specifically checking order safety:

```bash
toss --json order buy --symbol AAPL --qty 1 --type limit --price 1
```

Expected negative-gate behavior:

- The command must omit `--confirm`.
- The command must exit non-zero.
- The JSON error kind must be `validation`.
- No live order should be sent.

Do not run live mutating commands during verification.

## Order Safety

Treat these commands as mutating when not in dry-run mode:

- `toss order buy ...`
- `toss order sell ...`
- `toss order modify ...`
- `toss order cancel ...`

Safety invariants:

- Live mutating commands require `--confirm`.
- `--dry-run` takes precedence over `--confirm`.
- Create order supports `--client-order-id`; do not auto-generate it.
- High-value acknowledgement is separate: `--confirm-high-value-order` maps to Toss `confirmHighValueOrder` and does not replace `--confirm`.
- Create order must provide exactly one size field: `--qty` or `--amount`.
- There is no confirmed sandbox in the docs inspected so far. Assume live commands target production.

Before any live order, require the user to explicitly provide all live-order details in the current conversation: side, symbol, quantity or amount, order type, price when required, account, idempotency choice, and `--confirm` intent. Prefer asking the user to run live commands themselves after reviewing dry-run output.

## Development Rules

Use TDD for behavior changes. Add failing tests first, observe failure, implement the smallest fix, then rerun focused tests.

Keep wrappers thin:

1. Build query/body.
2. Call `TossClient`.
3. Parse typed `result`.

Keep financial values as strings or `serde_json::Value`; do not introduce `f32` or `f64` for money, prices, quantities, rates, commissions, or buying power.

Use mock transport tests for request construction and response parsing. Do not require real Toss credentials in automated tests.

Maintain stable CLI envelopes:

```json
{"ok":true,"command":"price","data":{}}
{"ok":false,"command":"price","error":{"kind":"api","message":"..."}}
```

JSON errors go to stdout. Text-mode errors go to stderr.

## Installation

Install release binary:

```bash
cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss --release
mkdir -p ~/.local/bin
install -m 755 rust/target/release/toss ~/.local/bin/toss
~/.local/bin/toss --json config
```

If shell lookup still finds an older binary, run `hash -r` or call `~/.local/bin/toss` directly.

## Additional Resources

Read `references/safety-checklist.md` before live-order work or credential verification involving real credentials.
