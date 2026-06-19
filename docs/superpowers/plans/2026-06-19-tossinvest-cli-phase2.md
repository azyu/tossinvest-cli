# Tossinvest CLI Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Phase 1's public read-only wrapper return types with typed `toss-core` request/response APIs while preserving the CLI's stable JSON/text behavior.

**Architecture:** Keep the Phase 1 transport/client/auth stack. Add typed model modules in `toss-core`, add generic typed response parsing to `TossClient`, then expose typed read-only wrapper functions while retaining `*_json` compatibility shims only where the CLI still needs raw `serde_json::Value` during migration. The CLI should convert typed results back through `serde_json::to_value` for existing output envelopes.

**Tech Stack:** Rust edition 2024, `serde`, `serde_json`, `tokio`, `async-trait`, `reqwest` with rustls, existing `toss-core`/`toss-cli` workspace.

## Global Constraints

- Use the OpenAPI document at `https://openapi.tossinvest.com/openapi-docs/latest/openapi.json` as the source of truth.
- Phase 2 implements typed wrapper/library core for the Phase 1 read-only API surface.
- Do not introduce order create/modify/cancel calls.
- Keep endpoint wrappers thin: build query/body, call client, parse `result`.
- Keep prices, quantities, and money values as strings or `serde_json::Value`; do not use floating-point types for financial values.
- Unknown enum-like API values must not fail deserialization; represent them as strings or transparent newtypes around `String`.
- Never print `client_secret` or access tokens in normal command output.
- JSON success/error envelopes remain stable for the CLI.
- Network behavior must remain testable through mock transport; tests must not require real Toss credentials.
- `.context` tracks status only; do not copy this whole plan into `.context`.

---

## File Structure

Create:

- `rust/toss-core/src/models/mod.rs` — public typed model module root.
- `rust/toss-core/src/models/common.rs` — `MoneyValue`, `DateString`, string-backed enum aliases/newtypes.
- `rust/toss-core/src/models/market_data.rs` — typed models for prices, orderbook, trades, price limits, and candles.
- `rust/toss-core/src/models/account.rs` — typed account model.
- `rust/toss-core/src/models/asset.rs` — typed holdings models.
- `rust/toss-core/src/models/market_info.rs` — typed exchange-rate and market-calendar models.
- `rust/toss-core/src/models/stock_info.rs` — typed stock info and warning models.

Modify:

- `rust/toss-core/src/lib.rs` — export `models`.
- `rust/toss-core/src/client.rs` — add generic typed parsing method.
- `rust/toss-core/src/market_data.rs` — expose typed wrapper functions and keep JSON compatibility where required.
- `rust/toss-core/src/account.rs` — expose typed account list wrapper.
- `rust/toss-core/src/asset.rs` — expose typed holdings wrapper.
- `rust/toss-core/src/market_info.rs` — expose typed market-info wrappers.
- `rust/toss-core/src/stock_info.rs` — expose typed stock wrappers.
- `rust/toss-cli/src/runtime.rs` — convert typed wrapper results to JSON for existing CLI output.
- `README.md` — document that `toss-core` exposes typed read-only wrappers.
- `.context/PROJECT.md`, `.context/STEERING.md`, `.context/TASKS.md` — update phase status only.

---

### Task 1: Typed Model Foundation and Client Parser

**Files:**
- Create: `rust/toss-core/src/models/mod.rs`
- Create: `rust/toss-core/src/models/common.rs`
- Modify: `rust/toss-core/src/lib.rs`
- Modify: `rust/toss-core/src/client.rs`

**Interfaces:**
- Consumes: `TossClient<T>::get_json(&self, path, query, account_required) -> Result<Value>`
- Produces: `TossClient<T>::get_typed<R>(&self, path: &str, query: Vec<(String, String)>, account_required: bool) -> Result<R> where R: DeserializeOwned`
- Produces: `toss_core::models::common::{MoneyValue, DateString, MarketCountry, Currency}`

- [ ] **Step 1: Write failing client typed parse test**

Add this test to `rust/toss-core/src/client.rs` tests:

```rust
#[derive(Debug, serde::Deserialize, PartialEq)]
struct TypedProbe {
    symbol: String,
    last_price: serde_json::Value,
}

#[tokio::test]
async fn parses_typed_result_without_floating_point() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(vec![
        HttpResponse { status: 200, headers: Vec::new(), body: br#"{"access_token":"token-1","token_type":"Bearer","expires_in":86400}"#.to_vec() },
        HttpResponse { status: 200, headers: Vec::new(), body: br#"{"result":{"symbol":"AAPL","last_price":"181.23"}}"#.to_vec() },
    ]));
    let transport = QueueTransport { requests, responses };
    let token_manager = TokenManager::new_with_cache_path(
        "client".to_string(),
        "secret".to_string(),
        tempfile::tempdir().unwrap().path().join("token.json"),
        transport.clone(),
    );
    let client = TossClient::new_with_parts(
        AppConfig { client_id: "client".to_string(), client_secret: "secret".to_string(), account_seq: None },
        token_manager,
        transport,
    );

    let typed: TypedProbe = client.get_typed("/api/v1/probe", Vec::new(), false).await.unwrap();
    assert_eq!(typed.symbol, "AAPL");
    assert_eq!(typed.last_price, serde_json::json!("181.23"));
}
```

- [ ] **Step 2: Run failing test**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core parses_typed_result_without_floating_point
```

Expected: fail because `get_typed` does not exist.

- [ ] **Step 3: Add model foundation**

Create `rust/toss-core/src/models/mod.rs`:

```rust
pub mod common;
```

Create `rust/toss-core/src/models/common.rs`:

```rust
use serde::{Deserialize, Serialize};

pub type MoneyValue = serde_json::Value;
pub type QuantityValue = serde_json::Value;
pub type DateString = String;
pub type TimestampString = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Currency(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MarketCountry(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountType(pub String);
```

Modify `rust/toss-core/src/lib.rs` to include:

```rust
pub mod models;
```

- [ ] **Step 4: Implement typed parser**

Modify `rust/toss-core/src/client.rs` imports:

```rust
use serde::de::DeserializeOwned;
```

Add this method inside `impl<T: Transport> TossClient<T>`:

```rust
pub async fn get_typed<R>(
    &self,
    path: &str,
    query: Vec<(String, String)>,
    account_required: bool,
) -> Result<R>
where
    R: DeserializeOwned,
{
    let value = self.get_json(path, query, account_required).await?;
    Ok(serde_json::from_value(value)?)
}
```

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core client::tests
```

Expected: all client tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add rust/toss-core/src/lib.rs rust/toss-core/src/client.rs rust/toss-core/src/models/mod.rs rust/toss-core/src/models/common.rs
git commit -m "feat: add typed response foundation"
```

---

### Task 2: Typed Market Data Wrappers

**Files:**
- Create: `rust/toss-core/src/models/market_data.rs`
- Modify: `rust/toss-core/src/models/mod.rs`
- Modify: `rust/toss-core/src/market_data.rs`

**Interfaces:**
- Consumes: `TossClient<T>::get_typed<R>()`
- Produces: `prices<T>(&TossClient<T>, &str) -> Result<Vec<PriceResponse>>`
- Produces: `orderbook<T>(&TossClient<T>, &str) -> Result<OrderbookResponse>`
- Produces: `trades<T>(&TossClient<T>, &str) -> Result<Vec<Trade>>`
- Produces: `price_limits<T>(&TossClient<T>, &str) -> Result<PriceLimitResponse>`
- Produces: `candles<T>(&TossClient<T>, Vec<(String, String)>) -> Result<CandlePageResponse>`
- Produces JSON compatibility functions with `_json` suffix for any CLI migration needs.

- [ ] **Step 1: Write failing typed market tests**

Add tests in `rust/toss-core/src/market_data.rs` that feed mocked `result` payloads and assert:

```rust
assert_eq!(prices[0].symbol, "AAPL");
assert_eq!(prices[0].last_price, serde_json::json!("181.23"));
assert_eq!(prices[0].currency.0, "USD");
```

Also assert unknown currency/country strings deserialize without failure.

- [ ] **Step 2: Run failing market tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core market_data::tests
```

Expected: fail because typed market data models/wrappers do not exist.

- [ ] **Step 3: Add market data models**

Create `rust/toss-core/src/models/market_data.rs` with serde structs using `MoneyValue`, `QuantityValue`, `Currency`, `MarketCountry`, and `Option<String>` for timestamps/dates. Include at least:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceResponse {
    pub symbol: String,
    #[serde(rename = "timestamp")]
    pub timestamp: Option<String>,
    #[serde(rename = "lastPrice")]
    pub last_price: MoneyValue,
    pub currency: Currency,
}
```

Define additional structs for `OrderbookResponse`, `OrderbookEntry`, `Trade`, `PriceLimitResponse`, `CandlePageResponse`, and `Candle` with public fields and serde renames matching OpenAPI camelCase names. Use `serde_json::Value` for unclear nested financial objects rather than floats.

Export from `rust/toss-core/src/models/mod.rs`:

```rust
pub mod market_data;
```

- [ ] **Step 4: Replace market wrapper return types**

Modify `rust/toss-core/src/market_data.rs`:

- Rename existing raw functions to `prices_json`, `orderbook_json`, `trades_json`, `price_limits_json`, `candles_json`.
- Add typed functions with the original names that call `client.get_typed` and return the model types.

- [ ] **Step 5: Run market tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core market_data::tests
```

Expected: tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add rust/toss-core/src/models/mod.rs rust/toss-core/src/models/market_data.rs rust/toss-core/src/market_data.rs
git commit -m "feat: type market data wrappers"
```

---

### Task 3: Typed Account, Asset, Stock, and Market Info Wrappers

**Files:**
- Create: `rust/toss-core/src/models/account.rs`
- Create: `rust/toss-core/src/models/asset.rs`
- Create: `rust/toss-core/src/models/stock_info.rs`
- Create: `rust/toss-core/src/models/market_info.rs`
- Modify: `rust/toss-core/src/models/mod.rs`
- Modify: `rust/toss-core/src/account.rs`
- Modify: `rust/toss-core/src/asset.rs`
- Modify: `rust/toss-core/src/stock_info.rs`
- Modify: `rust/toss-core/src/market_info.rs`

**Interfaces:**
- Produces typed wrappers for the rest of the Phase 1 API surface.
- Produces JSON compatibility functions with `_json` suffix for CLI migration.

- [ ] **Step 1: Write failing typed wrapper tests**

Add focused tests proving:

- `account::list` returns `Vec<Account>` and unknown `accountType` is preserved as `AccountType(String)`.
- `asset::holdings` returns a typed holdings response containing `HoldingsItem` with `quantity` as `serde_json::Value`.
- `stock_info::stocks` returns `Vec<StockInfo>` with unknown market/currency string values preserved.
- `market_info::exchange_rate` returns an `ExchangeRateResponse` without using floats.

- [ ] **Step 2: Run failing tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core account::tests asset::tests stock_info::tests market_info::tests
```

If Cargo rejects multiple filters, run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core
```

Expected: fail before implementation.

- [ ] **Step 3: Add models**

Create the four model files with serde structs. Use these rules:

- OpenAPI camelCase field names become Rust snake_case plus `#[serde(rename = "camelCase")]`.
- Money/quantity/rate values use `MoneyValue`, `QuantityValue`, or `serde_json::Value`.
- Unknown enum-like fields use transparent string newtypes from `models::common` or plain `String`.
- Nested financial summaries whose exact shape is broad may be `serde_json::Value` in Phase 2 if not needed by CLI rendering.

- [ ] **Step 4: Replace wrapper return types**

For each module, rename raw wrappers to `_json` and add typed wrappers with original names:

```rust
pub async fn list<T: Transport>(client: &TossClient<T>) -> Result<Vec<Account>> {
    client.get_typed("/api/v1/accounts", Vec::new(), false).await
}
```

Use the correct existing paths and account-bound flag from Phase 1.

- [ ] **Step 5: Run focused/core tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core
```

Expected: all `toss-core` tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add rust/toss-core/src/models rust/toss-core/src/account.rs rust/toss-core/src/asset.rs rust/toss-core/src/stock_info.rs rust/toss-core/src/market_info.rs
git commit -m "feat: type read only core wrappers"
```

---

### Task 4: Migrate CLI Runtime to Typed Core APIs

**Files:**
- Modify: `rust/toss-cli/src/runtime.rs`
- Modify: `rust/toss-cli/tests/cli_smoke.rs`

**Interfaces:**
- Consumes typed wrappers from Tasks 2-3.
- Produces unchanged CLI JSON/text envelopes.

- [ ] **Step 1: Write regression tests for stable CLI output**

Add or update tests proving:

- `toss --json config` still emits `ok`, `command`, and `data`.
- Existing parser smoke tests still pass.
- No order create/modify/cancel subcommands exist.

- [ ] **Step 2: Run CLI tests before migration**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-cli
```

Expected: current tests pass before migration.

- [ ] **Step 3: Convert typed results to JSON at runtime boundary**

In `rust/toss-cli/src/runtime.rs`, where read-only commands call `toss_core` wrappers, keep using original typed wrapper names and convert returned values to `serde_json::Value` only at the output boundary:

```rust
let value = market_data::prices(&client, symbols).await?;
write_output(&cli, command, serde_json::to_value(value)?, writer)
```

Do this for every Phase 1 command group.

- [ ] **Step 4: Remove unnecessary `_json` CLI use**

The CLI should not call `_json` compatibility functions unless a typed model is intentionally absent. If any `_json` call remains, document the exact reason in the task report.

- [ ] **Step 5: Run CLI and workspace tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-cli
cargo test --manifest-path rust/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add rust/toss-cli/src/runtime.rs rust/toss-cli/tests/cli_smoke.rs
git commit -m "refactor: use typed core in cli runtime"
```

---

### Task 5: Phase 2 Documentation, Context, and Verification

**Files:**
- Modify: `README.md`
- Modify: `.context/PROJECT.md`
- Modify: `.context/STEERING.md`
- Modify: `.context/TASKS.md`

**Interfaces:**
- Consumes completed typed core and CLI migration.
- Produces final Phase 2 verification evidence.

- [ ] **Step 1: Update docs and context**

Update `README.md` with a short library note:

```markdown
## Library core

`toss-core` exposes typed read-only wrappers for the Phase 1 API surface. Financial values are represented as `serde_json::Value` or strings instead of floating-point numbers.
```

Update `.context`:

- `PROJECT.md`: current state says Phase 2 typed core is complete and Phase 3 order CLI is next.
- `STEERING.md`: current priority shifts to Phase 3 only after final verification passes.
- `TASKS.md`: add a Phase 2 status section with task completion evidence.

- [ ] **Step 2: Run formatter**

Run:

```bash
cargo fmt --all --manifest-path rust/Cargo.toml
```

Expected: command exits successfully.

- [ ] **Step 3: Run all tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 4: Build binary**

Run:

```bash
cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss
```

Expected: build succeeds without warnings.

- [ ] **Step 5: Smoke-test config command**

Run with a temp config containing dummy non-secret-looking values:

```bash
tmp=$(mktemp -d) && cfg="$tmp/config.yaml" && printf 'client_id: "issued-client-id"\nclient_secret: "issued-client-secret"\n' > "$cfg" && cargo run --manifest-path rust/Cargo.toml -p toss-cli --bin toss -- --config "$cfg" --json config
```

Expected: command succeeds, prints `"ok":true`, masks `client_id`, and does not print `issued-client-secret`.

- [ ] **Step 6: Commit**

Run:

```bash
git add README.md .context/PROJECT.md .context/STEERING.md .context/TASKS.md rust/Cargo.lock rust/toss-core rust/toss-cli
git commit -m "docs: complete typed core phase"
```

---

## Self-Review

Spec coverage:

- Phase 2 typed public APIs are covered by Tasks 1-4.
- Financial value safety is covered by common model aliases and typed model tests.
- Unknown enum tolerance is covered by string-backed newtypes and tests.
- CLI envelope stability is covered by Task 4 regression tests.
- Phase 3 order commands remain excluded.

Placeholder scan:

- This plan contains exact files, commands, interfaces, and concrete implementation rules.
- The plan intentionally allows broad nested financial summaries to remain `serde_json::Value` because the design spec permits string or `serde_json::Value` in this phase.

Type consistency:

- `MoneyValue`, `QuantityValue`, `Currency`, `MarketCountry`, and `AccountType` are introduced in Task 1 before use.
- Typed wrappers keep the Phase 1 function names; raw compatibility functions use `_json` suffix consistently.
