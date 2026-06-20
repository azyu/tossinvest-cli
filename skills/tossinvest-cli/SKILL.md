---
name: tossinvest-cli
description: "Use when the user asks to use, install, configure, verify, or automate the `toss` CLI for Toss Securities Open API tasks: auth checks, account selection, holdings, market data, safe JSON parsing, dry-run orders, and live-order safety gates. This skill is for CLI operation and safety workflows, not Rust codebase development."
version: 0.1.0
author: Hermes Agent
license: MIT
platforms: [linux, macos, windows]
metadata:
  hermes:
    tags: [toss, tossinvest, brokerage, finance, cli, json, orders]
    related_skills: []
---

# Toss CLI Usage Skill

Use this skill to operate the installed `toss` command safely. Treat the Toss Securities Open API as production-capable brokerage infrastructure. Prefer read-only commands and dry-run order checks unless the user explicitly requests a live order and provides all required details in the current conversation.

This skill is for CLI usage, not repository development. Do not use it to decide Rust implementation patterns, tests, release automation, or crate architecture.

## Core Safety Rules

- Keep Toss credentials outside repositories and chat transcripts.
- Never print or repeat `client_secret`, OAuth access tokens, refresh tokens, or token cache contents.
- Treat account numbers, holdings, order history, buying power, and sellable quantity as private financial data.
- Summarize private payloads by status and shape unless the user explicitly asks for exact values.
- Run live mutating order commands only when the user explicitly asks for a live order in the current conversation.
- Prefer showing a dry-run command and asking the user to run the final live command locally.

For detailed order and credential safety checks, read `references/safety-checklist.md`.

## Installation Checks

Check whether `toss` is available:

```bash
toss --version
toss --help
toss --json config
```

If missing and Homebrew is available, install from the tap:

```bash
brew install azyu/tap/toss
```

If a local release binary was installed manually, prefer the explicit path when shell lookup may be stale:

```bash
~/.local/bin/toss --version
```

Run `hash -r` in POSIX shells after replacing an existing binary.

## Credential Setup

Use the default config path:

```text
~/.config/tossinvest/config.yaml
```

Create it with restrictive permissions:

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

Use environment variables for session-only configuration:

```bash
export TOSSINVEST_CLIENT_ID="issued-client-id"
export TOSSINVEST_CLIENT_SECRET="issued-client-secret"
export TOSSINVEST_ACCOUNT_SEQ="1"
```

Persist an account sequence only when account-bound commands are needed:

```bash
toss account list
toss account use 1
```

## Safe Verification Flow

Use this sequence for credential and installation checks. Stop at the first failure and report the command plus safe output shape, not private payload values.

```bash
toss --json config
toss --json auth token
toss --json account list
toss --json holdings
toss --json order buying-power --currency USD
toss --json order buy --symbol AAPL --qty 1 --type limit --price 1 --dry-run
```

Expected properties:

- `config` masks `client_id` and omits `client_secret`.
- `auth token` returns a token-check status without printing the token.
- Account-bound commands require a configured or overridden `account_seq`.
- Dry-run order output includes method, path, account header presence, and body shape, but does not send a live order.

## Common Read-only Commands

Use text output for human inspection:

```bash
toss price AAPL
toss price AAPL --symbols AAPL,MSFT
toss quote orderbook AAPL
toss quote trades AAPL
toss quote limits AAPL
toss chart candles AAPL --interval 1d
toss chart candles AAPL --interval 1m --from <FROM> --to <TO>

toss stock get AAPL
toss stock warnings 005930
toss stock search --symbols 005930,AAPL

toss market exchange-rate
toss market calendar kr
toss market calendar us

toss holdings
```

Use JSON output for automation and `--quiet` to reduce extra text in text mode:

```bash
toss --json price AAPL
toss --json holdings
toss --output json order list --status open
toss --quiet price AAPL
```

## JSON Output Handling

Successful JSON output uses this envelope:

```json
{"ok":true,"command":"price","data":{}}
```

Error JSON output uses this envelope:

```json
{"ok":false,"command":"price","error":{"kind":"api","message":"..."}}
```

In JSON mode, both success and error envelopes go to stdout. Failed JSON commands can still exit non-zero, so automation should check both the process status and the parsed envelope. In text mode, human output goes to stdout and errors go to stderr. Preserve `error.requestId` for debugging when present, but do not mix it with account or token data in public summaries.

## Order Safety Workflow

Use `--dry-run` first for every order path:

```bash
toss --json order buy --symbol AAPL --qty 1 --type limit --price 1 --dry-run
toss --json order sell --symbol AAPL --qty 1 --type market --dry-run
toss --json order modify <orderId> --type limit --price 180 --dry-run
toss --json order cancel <orderId> --dry-run
```

Before any live order, require all of these details in the current conversation:

- side: buy, sell, modify, or cancel
- symbol for create orders
- order ID for modify/cancel
- exactly one size input for create orders: `--qty` or `--amount`
- if using `--amount`, remember Toss docs constrain amount-based orders to US-market regular-hours cases
- order type: limit or market
- price when required by the order type
- account/accountSeq to use
- idempotency decision via `--client-order-id` when creating an order
- high-value acknowledgement decision if applicable
- explicit `--confirm` intent

Live mutating commands require `--confirm`. Treat the following as templates only; do not run them as examples:

```bash
toss order buy --symbol <SYMBOL> --qty <QTY> --type limit --price <PRICE> --client-order-id <CLIENT_ORDER_ID> --confirm
toss order sell --symbol <SYMBOL> --qty <QTY> --type market --confirm
toss order modify <ORDER_ID> --qty <QTY> --type limit --price <PRICE> --confirm --confirm-high-value-order
toss order cancel <ORDER_ID> --confirm
```

Never treat `--confirm-high-value-order` as a substitute for `--confirm`. Do not auto-generate `--client-order-id`; ask the user whether they want to provide one for idempotency.

## Additional Resources

- `references/safety-checklist.md` — credential handling, read-only verification, dry-run expectations, and live-order gates.
