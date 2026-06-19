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

Phase 1 is read-only. Order creation, modification, and cancellation are intentionally not exposed yet.
The CLI never prints `client_secret` or access tokens in normal output.
## Library core

`toss-core` exposes typed read-only wrappers for the Phase 1 API surface. Financial values are represented as `serde_json::Value` or strings instead of floating-point numbers.

