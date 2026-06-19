# toss

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> A small, agent-friendly Toss Securities Open API CLI built in Rust.

`toss` wraps the [Toss Securities Open API](https://developers.tossinvest.com/docs) with stable JSON envelopes, human-readable text output, and explicit safety gates for order commands.

## Features

- Read-only investment terminal for prices, quotes, charts, stocks, markets, accounts, and holdings
- Typed Rust core crate (`toss-core`) plus CLI crate (`toss-cli`)
- Human-friendly text output plus stable JSON output for automation
- Config file with environment overrides
- Dry-run order smoke checks
- Live order commands guarded by explicit `--confirm`
- Build metadata via `toss --version` and `toss -V`

## Installation

### Homebrew

```bash
brew install azyu/tap/toss
```

### From source

Requires Rust 1.93+.

```bash
cargo build --manifest-path rust/Cargo.toml -p toss-cli --release --bin toss
mkdir -p ~/.local/bin
install -m 755 rust/target/release/toss ~/.local/bin/toss
```

Verify the installed binary:

```bash
toss --version
```

Example output:

```text
toss version 0.1.0+<commit>
commit: <commit>
built: <UTC timestamp>
```

## Quick Start

### 1. Configure credentials

Create the default config file:

```bash
mkdir -p ~/.config/tossinvest
chmod 700 ~/.config/tossinvest
$EDITOR ~/.config/tossinvest/config.yaml
chmod 600 ~/.config/tossinvest/config.yaml
```

Config shape:

```yaml
client_id: "issued-client-id"
client_secret: "issued-client-secret"
```

Select and persist an account only when account-bound commands are needed:

```bash
toss account list
toss account use 1
```

You can also use session-only environment variables:

```bash
export TOSSINVEST_CLIENT_ID="issued-client-id"
export TOSSINVEST_CLIENT_SECRET="issued-client-secret"
export TOSSINVEST_ACCOUNT_SEQ="1"
```

### 2. Verify auth

```bash
toss --json config
toss --json auth token
```

`config` masks `client_id` and never prints `client_secret`. `auth token` checks token issuance and never prints the token.

### 3. Run read-only commands

```bash
toss price AAPL
toss quote orderbook AAPL
toss quote trades AAPL
toss chart candles AAPL --interval 1d

toss stock get AAPL
toss stock warnings 005930
toss stock search --symbols 005930,AAPL

toss market exchange-rate
toss market calendar kr
toss market calendar us

toss account list
toss account use 1
toss holdings
```

### 4. Run order safety checks

Dry-run order commands print the request shape and do not send a live order:

```bash
toss --json order buying-power --currency USD
toss --json order buy --symbol AAPL --qty 1 --type limit --price 1 --dry-run
```

Live mutating order commands require `--confirm`:

```bash
toss order buy --symbol AAPL --qty 1 --type limit --price 180 --client-order-id client-123 --confirm
toss order sell --symbol AAPL --qty 1 --type market --confirm
toss order modify ORD-123 --qty 2 --type limit --price 181 --confirm --confirm-high-value-order
toss order cancel ORD-123 --confirm
```

> [!CAUTION]
> No sandbox is documented in the inspected Toss Open API docs. Treat confirmed order commands as production brokerage traffic.

## Command Overview

| Group | Subcommands |
|-------|-------------|
| `toss config` | show resolved config summary |
| `toss auth` | `token` |
| `toss price` | current price by symbol |
| `toss quote` | `orderbook`, `trades`, `limits` |
| `toss chart` | `candles` |
| `toss stock` | `get`, `warnings`, `search` |
| `toss market` | `exchange-rate`, `calendar` |
| `toss account` | `list`, `use` |
| `toss holdings` | account holdings |
| `toss order` | `buy`, `sell`, `modify`, `cancel`, `list`, `show`, `buying-power`, `sellable-quantity`, `commissions` |

## Global Options

```text
--config <path>       config file (default: ~/.config/tossinvest/config.yaml)
--account <seq>       accountSeq override for account-bound commands
--output text|json    output format
--json                shortcut for --output json
--quiet               suppress extra text in text output
```

## Configuration and Auth

Config path priority:

1. `--config <path>`
2. `~/.config/tossinvest/config.yaml`

Environment overrides:

| Variable | Purpose |
|----------|---------|
| `TOSSINVEST_CLIENT_ID` | Toss Open API client ID |
| `TOSSINVEST_CLIENT_SECRET` | Toss Open API client secret |
| `TOSSINVEST_ACCOUNT_SEQ` | Optional default account sequence for account-bound commands |

Token cache path:

```text
~/.tossinvest/token.json
```

> [!CAUTION]
> Keep credentials and token cache files outside the repository. Use restrictive file permissions.

## Output Contract

Use `--json` or `--output json` for automation.

Successful JSON output:

```json
{"ok":true,"command":"price","data":{}}
```

Error JSON output:

```json
{"ok":false,"command":"price","error":{"kind":"api","code":"stock-not-found","message":"...","requestId":"..."}}
```

Text output is for humans. In text mode, command errors are written to stderr. In JSON mode, success and error envelopes are written to stdout.

## Order Safety

- `--dry-run` takes precedence over `--confirm`.
- Live `buy`, `sell`, `modify`, and `cancel` require `--confirm`.
- Create order accepts `--client-order-id`; generate your own idempotency key when needed.
- Create order must provide exactly one size field: `--qty` or `--amount`.
- `--confirm-high-value-order` maps to Toss `confirmHighValueOrder`; it does not replace `--confirm`.
- Prices, quantities, money values, and rates are kept as strings or JSON values, not floating-point numbers.

## Developer Docs

- Design spec: [`docs/superpowers/specs/2026-06-18-tossinvest-cli-design.md`](docs/superpowers/specs/2026-06-18-tossinvest-cli-design.md)
- Phase 1 plan: [`docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md`](docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md)
- Phase 2 plan: [`docs/superpowers/plans/2026-06-19-tossinvest-cli-phase2.md`](docs/superpowers/plans/2026-06-19-tossinvest-cli-phase2.md)
- Phase 3 plan: [`docs/superpowers/plans/2026-06-19-tossinvest-cli-phase3.md`](docs/superpowers/plans/2026-06-19-tossinvest-cli-phase3.md)
