# toss

[![CI](https://github.com/azyu/tossinvest-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/azyu/tossinvest-cli/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/azyu/tossinvest-cli)](https://github.com/azyu/tossinvest-cli/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[English](README.md) | [한국어](README.ko.md)

> A small, agent-friendly Toss Securities Open API CLI built in Rust.

## Features

- Read-only commands for prices, quotes, charts, stocks, markets, accounts, and holdings
- Order commands with dry-run output and explicit `--confirm` safety gates
- Human-friendly text output plus stable JSON output for automation
- Config file with environment overrides
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

```bash
mkdir -p ~/.config/tossinvest
chmod 700 ~/.config/tossinvest
$EDITOR ~/.config/tossinvest/config.yaml
chmod 600 ~/.config/tossinvest/config.yaml
```

```yaml
client_id: "issued-client-id"
client_secret: "issued-client-secret"
```

You can also use environment variables:

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

`account_seq` is optional until account-bound commands are used.

### 3. Run common read-only commands

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

toss holdings
```

### 4. Check order safety

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
| `toss --version` / `toss -V` | build metadata |

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
