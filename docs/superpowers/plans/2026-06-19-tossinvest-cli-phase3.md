# Tossinvest CLI Phase 3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add real order-capable CLI support with dry-run, explicit confirmation, idempotency, order-info helpers, and tested request construction.

**Architecture:** Extend the existing typed `toss-core` client with typed POST support and order/order-info wrappers. Add CLI order commands that default to non-mutating behavior unless the user explicitly confirms live submission. Keep all money/quantity values as strings or `serde_json::Value`, and keep CLI JSON/text envelopes stable.

**Tech Stack:** Rust edition 2024, existing `toss-core`/`toss-cli`, `serde`, `serde_json`, `clap`, `tokio`, mockable `Transport` tests.

## Global Constraints

- Use the OpenAPI document at `https://openapi.tossinvest.com/openapi-docs/latest/openapi.json` as the source of truth.
- Phase 3 may expose order create/modify/cancel only with dry-run and confirmation safety in place.
- Public Toss docs inspected so far show a single production server; sandbox/staging support is not assumed.
- Mutating order commands must support `--dry-run`.
- Mutating order commands must require explicit `--confirm` for live submission.
- `clientOrderId` must be supported for create order idempotency.
- The CLI must not auto-generate `clientOrderId` unless the user opts in; this plan does not add auto-generation.
- Large-order confirmation must expose Toss `confirmHighValueOrder` behavior explicitly via a flag.
- Dry-run output must include method, path, account header presence, and request body with secrets omitted.
- Keep endpoint wrappers thin: build query/body, call client, parse `result`.
- Keep prices, quantities, and money values as strings or `serde_json::Value`; do not use floating-point types.
- Never print `client_secret` or access tokens in normal command output.
- JSON mode must use stable success/error envelopes.
- Text mode prints human output to stdout and errors to stderr.
- Network behavior must remain testable through mock transport; tests must not require real Toss credentials.
- `.context` tracks status only; do not copy this full plan into `.context`.

---

## Credential Guidance

Real Toss credentials are useful now for manual read-only smoke checks, account discovery, and later live order smoke tests. They must stay outside the repository.

Recommended local setup:

```bash
mkdir -p ~/.config/tossinvest
chmod 700 ~/.config/tossinvest
cat > ~/.config/tossinvest/config.yaml <<'EOF'
client_id: "issued-client-id"
client_secret: "issued-client-secret"
account_seq: 1
EOF
chmod 600 ~/.config/tossinvest/config.yaml
```

Alternative session-only setup:

```bash
export TOSSINVEST_CLIENT_ID="issued-client-id"
export TOSSINVEST_CLIENT_SECRET="issued-client-secret"
export TOSSINVEST_ACCOUNT_SEQ="1"
```

Never paste real credentials into chat, commit them, or put them under this repository. Run read-only validation first:

```bash
cargo run --manifest-path rust/Cargo.toml -p toss-cli --bin toss -- --json auth token
cargo run --manifest-path rust/Cargo.toml -p toss-cli --bin toss -- --json account list
cargo run --manifest-path rust/Cargo.toml -p toss-cli --bin toss -- --json holdings
```

Do not run live mutating order commands until dry-run output, confirmation behavior, and idempotency behavior pass tests.

---

## File Structure

Create:

- `rust/toss-core/src/models/order.rs` — order request/response models.
- `rust/toss-core/src/models/order_info.rs` — buying power, sellable quantity, commission models.
- `rust/toss-core/src/order.rs` — create/modify/cancel wrappers and dry-run request builders.
- `rust/toss-core/src/order_info.rs` — order-info read-only wrappers.

Modify:

- `rust/toss-core/src/lib.rs` — export new modules.
- `rust/toss-core/src/client.rs` — add typed POST support and reusable dry-run request construction.
- `rust/toss-cli/src/cli.rs` — add order command parser and flags.
- `rust/toss-cli/src/runtime.rs` — dispatch order-info and order mutation commands.
- `rust/toss-cli/tests/cli_smoke.rs` — parser, dry-run, confirmation, and no-secret smoke tests.
- `README.md` — document order safety and live-command warnings.
- `.context/PROJECT.md`, `.context/STEERING.md`, `.context/TASKS.md` — status only.

---

### Task 1: Core POST Client and Order Models

**Files:**
- Modify: `rust/toss-core/src/client.rs`
- Modify: `rust/toss-core/src/lib.rs`
- Create: `rust/toss-core/src/models/order.rs`
- Modify: `rust/toss-core/src/models/mod.rs`

**Interfaces:**
- Produces: `TossClient<T>::post_typed<R, B>(&self, path: &str, body: &B, account_required: bool) -> Result<R>` where `R: DeserializeOwned`, `B: Serialize`.
- Produces: `TossClient<T>::build_post_request<B>(&self, path: &str, body: &B, account_required: bool, include_auth: bool) -> Result<HttpRequest>` for dry-run/test construction.
- Produces order models: `OrderCreateRequest`, `OrderModifyRequest`, `OrderResponse`, `OrderSide`, `OrderType`, `TimeInForce`.

- [ ] **Step 1: Write failing client POST tests**

Add tests in `rust/toss-core/src/client.rs` verifying:

- `post_typed` sends `POST`, `content-type: application/json`, bearer token, account header, and JSON body.
- `build_post_request(..., include_auth=false)` returns a request body without fetching token and without authorization header.
- Missing account for account-bound POST returns `TossError::Validation` before token fetch.

- [ ] **Step 2: Run failing client POST tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core client::tests
```

Expected: fail because POST support does not exist.

- [ ] **Step 3: Add order models**

Create `rust/toss-core/src/models/order.rs` with serde models:

- `OrderCreateRequest`
  - `client_order_id: Option<String>` → `clientOrderId`, skip if none
  - `symbol: String`
  - `side: OrderSide`
  - `order_type: OrderType` → `orderType`
  - `time_in_force: Option<TimeInForce>` → `timeInForce`, skip if none
  - `quantity: Option<serde_json::Value>`, skip if none
  - `price: Option<serde_json::Value>`, skip if none
  - `confirm_high_value_order: Option<bool>` → `confirmHighValueOrder`, skip if none
  - `order_amount: Option<serde_json::Value>` → `orderAmount`, skip if none
- `OrderModifyRequest`
  - `order_type: OrderType`
  - `quantity: Option<serde_json::Value>`
  - `price: Option<serde_json::Value>`
  - `confirm_high_value_order: Option<bool>`
- string-backed enums with exact serialized values: `BUY`, `SELL`, `LIMIT`, `MARKET`, `DAY`, `CLS`.
- `OrderResponse` with broad string/value fields for server response; opaque ids stay `String`.

- [ ] **Step 4: Implement POST support**

In `rust/toss-core/src/client.rs`:

- Add `post_typed` using existing token/account/error parsing.
- Add `build_post_request` for dry-run construction.
- Add a helper to serialize JSON body exactly once.
- Ensure `include_auth=false` never fetches token and never adds authorization header.
- Ensure account-bound missing account returns validation before token fetch.

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core client::tests
```

Expected: all client tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add rust/toss-core/src/client.rs rust/toss-core/src/lib.rs rust/toss-core/src/models/mod.rs rust/toss-core/src/models/order.rs
git commit -m "feat: add typed order post foundation"
```

---

### Task 2: Core Order and Order-Info Wrappers

**Files:**
- Create: `rust/toss-core/src/order.rs`
- Create: `rust/toss-core/src/order_info.rs`
- Create: `rust/toss-core/src/models/order_info.rs`
- Modify: `rust/toss-core/src/lib.rs`
- Modify: `rust/toss-core/src/models/mod.rs`

**Interfaces:**
- Produces: `order::create`, `order::modify`, `order::cancel` live wrappers.
- Produces: `order::build_create_dry_run`, `order::build_modify_dry_run`, `order::build_cancel_dry_run` request builders.
- Produces: `order_info::buying_power`, `order_info::sellable_quantity`, `order_info::commissions` typed read-only wrappers.

- [ ] **Step 1: Write failing wrapper tests**

Add mock-transport tests proving:

- create order posts to `/api/v1/orders` with account header and typed body.
- modify order posts to `/api/v1/orders/{orderId}/modify`.
- cancel order posts to `/api/v1/orders/{orderId}/cancel` with `{}` or omitted object per implemented request shape.
- dry-run builders do not include authorization header or token value.
- order-info wrappers call GET paths with account header.

- [ ] **Step 2: Run failing tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core order::tests order_info::tests
```

If Cargo rejects multiple filters, run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core
```

Expected: fail before implementation.

- [ ] **Step 3: Implement wrappers**

Implement thin wrappers using `TossClient::post_typed`, `TossClient::build_post_request`, and existing `get_typed`.

Paths:

- `POST /api/v1/orders`
- `POST /api/v1/orders/{orderId}/modify`
- `POST /api/v1/orders/{orderId}/cancel`
- `GET /api/v1/buying-power?currency=...`
- `GET /api/v1/sellable-quantity?symbol=...`
- `GET /api/v1/commissions`

- [ ] **Step 4: Run focused/core tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core
```

Expected: all core tests pass.

- [ ] **Step 5: Commit**

Run:

```bash
git add rust/toss-core/src/order.rs rust/toss-core/src/order_info.rs rust/toss-core/src/models/order_info.rs rust/toss-core/src/lib.rs rust/toss-core/src/models/mod.rs
git commit -m "feat: add order core wrappers"
```

---

### Task 3: CLI Order Parser and Safety Validation

**Files:**
- Modify: `rust/toss-cli/src/cli.rs`
- Modify: `rust/toss-cli/tests/cli_smoke.rs`

**Interfaces:**
- Produces order command parser without live dispatch yet.
- Enforces `--dry-run`/`--confirm` parser shape and clientOrderId availability.

- [ ] **Step 1: Write failing parser tests**

Add tests proving:

- `toss order buying-power --currency USD` parses.
- `toss order sellable-quantity --symbol AAPL` parses.
- `toss order commissions` parses.
- `toss order buy --symbol AAPL --qty 1 --type limit --price 180 --dry-run` parses.
- `toss order buy --symbol AAPL --amount 100 --type market --dry-run` parses.
- `toss order sell --symbol AAPL --qty 1 --type market --dry-run` parses.
- `toss order modify opaque-id --type limit --price 180 --dry-run` parses.
- `toss order cancel opaque-id --dry-run` parses.
- No mutating order command requires live network unless `--confirm` is present at runtime; parser accepts both `--dry-run` and `--confirm`, but runtime decides safety.

- [ ] **Step 2: Run failing parser tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-cli order_
```

Expected: fail because order parser does not exist.

- [ ] **Step 3: Add CLI parser types**

Add `Command::Order(OrderArgs)` with subcommands:

- `buy`
- `sell`
- `modify`
- `cancel`
- `buying-power`
- `sellable-quantity`
- `commissions`

Mutating command flags:

- `--dry-run`
- `--confirm`
- `--client-order-id <id>` for create only
- `--confirm-high-value-order`

Use strings for `qty`, `amount`, and `price`.

Use clap value enums for `OrderType`, `OrderSide` when helpful, but serialize to Toss values later.

- [ ] **Step 4: Run parser tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-cli order_
```

Expected: parser tests pass.

- [ ] **Step 5: Commit**

Run:

```bash
git add rust/toss-cli/src/cli.rs rust/toss-cli/tests/cli_smoke.rs
git commit -m "feat: add order cli parser"
```

---

### Task 4: CLI Dry-Run and Order-Info Dispatch

**Files:**
- Modify: `rust/toss-cli/src/runtime.rs`
- Modify: `rust/toss-cli/tests/cli_smoke.rs`

**Interfaces:**
- Produces live read-only order-info dispatch.
- Produces mutating order dry-run output.
- Live create/modify/cancel still rejected without `--confirm`.

- [ ] **Step 1: Write failing runtime tests**

Add tests proving:

- `order buy ... --dry-run --json` returns `ok:true` with method/path/account-header-present/body and no token/secret.
- `order cancel ... --dry-run --json` returns method/path/account-header-present/body.
- `order buy ...` without `--dry-run` and without `--confirm` returns validation error.
- `order buying-power --currency USD --json` dispatches through typed order-info wrapper under mock transport if runtime has mockable injection; otherwise parser/smoke coverage plus core tests are acceptable for this task.

- [ ] **Step 2: Run failing runtime tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-cli order_
```

Expected: fail before runtime implementation.

- [ ] **Step 3: Implement dry-run output**

In runtime:

- Build order request models from CLI args.
- For `--dry-run`, call core dry-run builders with `include_auth=false`.
- Output JSON data:

```json
{
  "dryRun": true,
  "method": "POST",
  "path": "/api/v1/orders",
  "accountHeaderPresent": true,
  "body": {}
}
```

- Do not include `Authorization`, token, `client_secret`, or `client_id`.
- If neither `--dry-run` nor `--confirm` is present for mutating commands, return validation error.

- [ ] **Step 4: Wire order-info commands**

Wire:

- `order buying-power`
- `order sellable-quantity`
- `order commissions`

These are read-only and do not need `--dry-run` or `--confirm`.

- [ ] **Step 5: Run CLI tests**

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
git commit -m "feat: add order dry run cli"
```

---

### Task 5: Live Order Dispatch with Confirmation

**Files:**
- Modify: `rust/toss-cli/src/runtime.rs`
- Modify: `rust/toss-cli/tests/cli_smoke.rs`

**Interfaces:**
- Produces live order dispatch only when `--confirm` is present and `--dry-run` is absent.
- Keeps `--dry-run` precedence over `--confirm` if both are present.

- [ ] **Step 1: Write failing confirmation tests**

Add tests proving:

- `order buy ... --confirm --json` dispatches live wrapper under mock transport and returns JSON envelope.
- `order buy ... --confirm --dry-run --json` returns dry-run output and does not include auth/token.
- `clientOrderId` is serialized when provided.
- `confirmHighValueOrder` is serialized only when flag is present.

- [ ] **Step 2: Run failing tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-cli order_
```

Expected: fail before live dispatch implementation.

- [ ] **Step 3: Implement live dispatch**

For create/modify/cancel:

- If `--dry-run`: dry-run path.
- Else if `--confirm`: call live wrapper.
- Else: validation error.

Error message must say live orders require `--confirm` or use `--dry-run`.

- [ ] **Step 4: Run CLI/workspace tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-cli
cargo test --manifest-path rust/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

Run:

```bash
git add rust/toss-cli/src/runtime.rs rust/toss-cli/tests/cli_smoke.rs
git commit -m "feat: gate live order commands"
```

---

### Task 6: Documentation, Context, and Final Verification

**Files:**
- Modify: `README.md`
- Modify: `.context/PROJECT.md`
- Modify: `.context/STEERING.md`
- Modify: `.context/TASKS.md`

**Interfaces:**
- Produces final Phase 3 documentation and verification evidence.

- [ ] **Step 1: Update README order section**

Document:

- Order-info commands are read-only.
- Mutating order commands support `--dry-run`.
- Live mutating order commands require `--confirm`.
- `--client-order-id` is recommended for create order idempotency.
- `--confirm-high-value-order` maps to Toss `confirmHighValueOrder`.
- There is no documented sandbox; assume production.

- [ ] **Step 2: Update `.context` status**

Update `.context` as status only:

- `PROJECT.md`: Phase 3 implemented after final verification.
- `STEERING.md`: order safety decisions and live production warning.
- `TASKS.md`: Phase 3 task status and verification evidence.

- [ ] **Step 3: Run formatter**

Run:

```bash
cargo fmt --all --manifest-path rust/Cargo.toml
```

Expected: command exits successfully.

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 5: Build binary**

Run:

```bash
cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss
```

Expected: build succeeds without warnings.

- [ ] **Step 6: Run non-mutating smoke checks**

Run config smoke with temp config:

```bash
tmp=$(mktemp -d) && cfg="$tmp/config.yaml" && printf 'client_id: "issued-client-id"\nclient_secret: "issued-client-secret"\naccount_seq: 1\n' > "$cfg" && cargo run --manifest-path rust/Cargo.toml -p toss-cli --bin toss -- --config "$cfg" --json config
```

Run dry-run smoke without real credentials:

```bash
cargo run --manifest-path rust/Cargo.toml -p toss-cli --bin toss -- --config "$cfg" --json order buy --symbol AAPL --qty 1 --type limit --price 180 --dry-run
```

Expected: both succeed, no secret/token output, dry-run output shows method/path/body/account-header-present.

- [ ] **Step 7: Commit**

Run:

```bash
git add README.md .context/PROJECT.md .context/STEERING.md .context/TASKS.md rust/Cargo.lock rust/toss-core rust/toss-cli
git commit -m "docs: complete order cli phase"
```

---

## Self-Review

Spec coverage:

- Dry-run and confirmation requirements are covered by Tasks 3-5.
- Idempotency support is covered by `clientOrderId` in Tasks 1, 3, and 5.
- `confirmHighValueOrder` support is covered by Tasks 1, 3, and 5.
- Order-info read-only helpers are covered by Task 2 and Task 4.
- No sandbox assumption is documented in Task 6.

Placeholder scan:

- This plan uses concrete paths, commands, flags, and output shapes.
- It does not require real credentials for tests.

Type consistency:

- Core order models are introduced before wrappers.
- CLI parser flags map directly to core request model fields.
- Dry-run and live dispatch share request construction to avoid drift.
