# tossinvest-cli

Rust CLI for Toss Securities Open API. The binary name is `toss`.

## Install

```bash
cargo build --manifest-path rust/Cargo.toml -p toss-cli --release --bin toss
install -m 755 rust/target/release/toss ~/.local/bin/toss
```

## Config

Default config path:

```text
~/.config/tossinvest/config.yaml
```

Example:

```yaml
client_id: "issued-client-id"
client_secret: "issued-client-secret"
account_seq: 1
```

Environment overrides:

```bash
export TOSSINVEST_CLIENT_ID="issued-client-id"
export TOSSINVEST_CLIENT_SECRET="issued-client-secret"
export TOSSINVEST_ACCOUNT_SEQ="1"
```

## Commands

```bash
toss config
toss --json config

toss price 005930
toss price AAPL
toss quote orderbook AAPL
toss quote trades AAPL
toss quote limits 005930
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

## Orders

Read-only order-info commands:

```bash
toss order buying-power --currency USD
toss order sellable-quantity --symbol AAPL
toss order commissions
```

Mutating order commands support `--dry-run` for smoke checks. Live mutating order commands require `--confirm`.

```bash
toss order buy --symbol AAPL --qty 1 --type limit --price 180 --client-order-id client-123 --dry-run
toss order buy --symbol AAPL --qty 1 --type limit --price 180 --confirm
toss order modify ORD-123 --qty 2 --type limit --price 181 --confirm --confirm-high-value-order
toss order cancel ORD-123 --dry-run
```

- `--client-order-id` is recommended for create-order idempotency.
- `--confirm-high-value-order` maps to Toss `confirmHighValueOrder`.
- There is no documented sandbox; assume production.

## Output

Use `--json` or `--output json` for automation. Successful JSON output uses:

```json
{"ok":true,"command":"price","data":{}}
```

Error JSON output uses:

```json
{"ok":false,"command":"price","error":{"kind":"api","code":"stock-not-found","message":"...","requestId":"..."}}
```

## Safety

The CLI never prints `client_secret` or access tokens in normal output.

## Library core

`toss-core` exposes typed read-only wrappers for the Phase 1 API surface. Financial values are represented as `serde_json::Value` or strings instead of floating-point numbers.

