# toss

[![CI](https://github.com/azyu/tossinvest-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/azyu/tossinvest-cli/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/azyu/tossinvest-cli)](https://github.com/azyu/tossinvest-cli/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[English](README.md) | [한국어](README.ko.md)

> A small, agent-friendly Toss Securities Open API CLI built in Rust.

## Features

- Read-only commands for prices, quotes, charts, stocks, markets, account listing, and holdings
- Order commands with dry-run output and explicit `--confirm` safety gates
- Human-friendly text output plus stable JSON output for automation
- Guided `toss setup`, config file, and environment override support
- Cross-platform release binaries for Linux, macOS, and Windows
- Build metadata via `toss --version` and `toss -V`

## Installation

### Homebrew

```bash
brew install azyu/tap/toss
```

### Prebuilt binaries

Download the latest archive from [GitHub Releases](https://github.com/azyu/tossinvest-cli/releases/latest).

| Platform | Asset |
|----------|-------|
| Linux amd64 | `toss_0.x.y_linux_amd64.tar.gz` |
| Linux arm64 | `toss_0.x.y_linux_arm64.tar.gz` |
| macOS arm64 | `toss_0.x.y_macos_arm64.tar.gz` |
| Windows x64 | `toss_0.x.y_windows_x64.zip` |
| Windows arm64 | `toss_0.x.y_windows_arm64.zip` |

### From source

Requires Rust 1.93+.

```bash
cargo build --manifest-path rust/Cargo.toml -p toss-cli --release --bin toss
mkdir -p ~/.local/bin
install -m 755 rust/target/release/toss ~/.local/bin/toss
```

## Quick Start

### 1. Configure credentials

The recommended path is the interactive setup command:

```bash
toss setup
```

For scripted setup, keep the secret out of command-line arguments and pass it on stdin:

```bash
printf '%s\n' "$TOSSINVEST_CLIENT_SECRET" | \
  toss setup --client-id "$TOSSINVEST_CLIENT_ID" --with-secret-stdin --no-check
```

`toss setup` writes `~/.config/tossinvest/config.yaml` with restrictive permissions on Unix. The file contains plaintext credentials, so keep it outside repositories and backups you do not trust.

You can also use environment variables without writing a config file:

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

#### 2.1. Select an account when needed (optional)

```bash
toss account list
toss account use 1
```

`account_seq` is optional until account-bound commands are used. `toss account use` writes the selected sequence to the local config file. Use `--account <seq>` for one-off overrides.

### 3. Run common read-only commands

```bash
toss price AAPL
toss quote orderbook AAPL
toss quote trades AAPL
toss chart candles AAPL --interval 1d --count 200

toss stock get AAPL
toss stock warnings 005930
toss stock search --symbols 005930,AAPL

toss market exchange-rate
toss market calendar kr
toss market calendar us

toss holdings
```

For candle pagination, pass the previous response's `nextBefore` as `--before`:

```bash
toss chart candles AAPL --interval 1m --count 200 --before "2026-06-19T18:20:00+09:00"
```

### 4. Check order safety

Read-only order/account info commands call Toss APIs but do not create, modify, or cancel orders:

```bash
toss --json order buying-power --currency USD
toss --json order sellable-quantity --symbol AAPL
toss --json order commissions
toss --json order list --status open
toss --json order show <orderId>
```

Dry-run mutating order commands print the request shape and do not send a live order:

```bash
toss --json order buy --symbol AAPL --qty 1 --type limit --price 1 --dry-run
```

Live mutating order commands require `--confirm`. Treat these as templates, not examples to run as-is:

```bash
toss order buy --symbol <SYMBOL> --qty <QTY> --type limit --price <PRICE> --client-order-id <CLIENT_ORDER_ID> --confirm
toss order sell --symbol <SYMBOL> --qty <QTY> --type market --confirm
toss order modify <ORDER_ID> --qty <QTY> --type limit --price <PRICE> --confirm --confirm-high-value-order
toss order cancel <ORDER_ID> --confirm
```

> [!CAUTION]
> No sandbox is documented in the inspected Toss Open API docs. Treat confirmed order commands as production brokerage traffic.

## Command Overview

| Group | Subcommands |
|-------|-------------|
| `toss setup` | save `client_id` and `client_secret` to the local config file |
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
| `toss --version` / `toss -V` | build metadata |

> [!NOTE]
> `toss setup` writes plaintext credentials to the local config file; it never prints `client_secret`.
> `toss account use <seq>` updates only the account sequence in the same local config file. Use `--account <seq>` for a one-off account override.
> `toss order list` requires `--status open|closed`; `toss order show` requires an order ID; `toss order sellable-quantity` requires `--symbol`.

## Configuration and Auth

Config path priority:

1. `--config <path>`
2. `~/.config/tossinvest/config.yaml`

Credential setup commands:

```bash
# Interactive: prompts for client_id and client_secret.
toss setup

# Scripted: reads client_secret from stdin, not from argv.
printf '%s\n' "$TOSSINVEST_CLIENT_SECRET" | \
  toss setup --client-id "$TOSSINVEST_CLIENT_ID" --with-secret-stdin --no-check

# One-time account selection while writing credentials.
printf '%s\n' "$TOSSINVEST_CLIENT_SECRET" | \
  toss setup --client-id "$TOSSINVEST_CLIENT_ID" --with-secret-stdin --account 1
```

`toss setup` runs a token check after saving unless `--no-check` is set. Use `--no-check` for offline setup, CI smoke tests, or when credentials are not yet active.

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
> `toss setup` stores `client_secret` in the local config file as plaintext. Keep credentials and token cache files outside the repository. Use restrictive file permissions or environment variables when local plaintext storage is not acceptable.

## Output Contract

Use `--json` or `--output json` for automation.

```json
{"ok":true,"command":"price","data":{}}
```

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

- [Technical spec](docs/superpowers/specs/2026-06-18-tossinvest-cli-design.md)
- [Phase 1 plan](docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md)
- [Phase 2 plan](docs/superpowers/plans/2026-06-19-tossinvest-cli-phase2.md)
- [Phase 3 plan](docs/superpowers/plans/2026-06-19-tossinvest-cli-phase3.md)
