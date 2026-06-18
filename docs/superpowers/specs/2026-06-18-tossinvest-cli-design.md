# Toss Securities Open API Wrapper CLI Design

## Goal

Build a Rust CLI for Toss Securities Open API using the same user-facing discipline as `kis-cli`, but with a Toss-specific implementation shape.

The delivery order is:

1. Read-only investment terminal.
2. Typed wrapper/library core.
3. Real order-capable CLI.

## Non-goals

- Do not copy KIS-specific TR-ID, hashkey, virtual/real environment, or WebSocket concepts.
- Do not implement unsupported Toss WebSocket behavior.
- Do not expose order commands before dry-run, confirmation, and idempotency behavior are defined and tested.
- Do not treat the CLI as a raw curl wrapper, except for an optional diagnostic `api` command later.

## Source of truth

Toss API behavior comes from the server-owned OpenAPI document:

- `https://openapi.tossinvest.com/openapi-docs/latest/openapi.json`

Human-readable references:

- `https://openapi.tossinvest.com/openapi-docs/overview.md`
- `https://openapi.tossinvest.com/openapi-docs/latest/api-reference/README.md`

The OpenAPI document is the authority for endpoints, schemas, request bodies, response envelopes, and documented errors.

## Architecture

Use a two-crate Rust workspace:

```text
rust/
├── toss-core/   # config, auth, HTTP client, typed endpoint wrappers, API models
└── toss-cli/    # clap parser, command dispatch, text/json rendering, process behavior
```

### `toss-core`

Responsibilities:

- Load config from file and environment.
- Issue OAuth2 Client Credentials tokens.
- Cache access tokens until shortly before expiry.
- Serialize calls through a shared HTTP client.
- Attach `Authorization: Bearer {access_token}` to all non-token calls.
- Attach `X-Tossinvest-Account` only for account, asset, order, order history, and order info APIs that require it.
- Parse standard Toss success responses through `result`.
- Parse standard Toss errors through `error`.
- Parse `/oauth2/token` OAuth errors separately.
- Preserve rate-limit and request-id headers in error metadata where available.

Config fields:

```yaml
client_id: "..."
client_secret: "..."
account_seq: 1   # optional until account-bound commands are used
```

Environment overrides:

```bash
TOSSINVEST_CLIENT_ID
TOSSINVEST_CLIENT_SECRET
TOSSINVEST_ACCOUNT_SEQ
```

Default config path:

```text
~/.config/tossinvest/config.yaml
```

Token cache path:

```text
~/.tossinvest/token.json
```

Token cache files must be written with owner-only permissions where the platform supports it.

### `toss-cli`

Responsibilities:

- Provide stable human commands rather than mirroring OpenAPI operation names directly.
- Support global flags:
  - `--config <path>`
  - `--output text|json`
  - `--json` as an alias for `--output json`
  - `--quiet` for text output only
  - `--account <accountSeq>` to override config for account-bound commands
- Print successful JSON responses in a stable envelope.
- Print JSON errors to stdout when JSON output is selected.
- Print text errors to stderr in text mode.
- Keep order commands unavailable until the order phase.

JSON success envelope:

```json
{
  "ok": true,
  "command": "price",
  "data": {}
}
```

JSON error envelope:

```json
{
  "ok": false,
  "command": "price",
  "error": {
    "kind": "api",
    "code": "stock-not-found",
    "message": "...",
    "requestId": "..."
  }
}
```

Error kinds:

- `validation`: CLI argument/config validation failed.
- `config`: config file or credential setup failed.
- `auth`: token issuance or OAuth error failed.
- `api`: Toss API returned an error envelope or non-success HTTP status.
- `rate_limit`: Toss API returned HTTP 429.
- `runtime`: network, serialization, or unexpected runtime failure.

## Phase 1: read-only investment terminal

Commands:

```bash
toss config
toss auth token

toss price <symbol> [--symbols <csv>]
toss quote orderbook <symbol>
toss quote trades <symbol>
toss quote limits <symbol>
toss chart candles <symbol> --interval <1m|1d> [--from <date>] [--to <date>]

toss stock get <symbol>
toss stock warnings <symbol>
toss stock search --symbols <csv>

toss market exchange-rate
toss market calendar kr
toss market calendar us

toss account list
toss account use <accountSeq>
toss holdings [--account <accountSeq>]
```

Phase 1 excludes create/modify/cancel order calls.

Account behavior:

- `account list` does not require an account header.
- `account use <accountSeq>` writes the selected account to the config file.
- `holdings` requires an account from `--account`, config, or `TOSSINVEST_ACCOUNT_SEQ`.
- If an account-bound command has no account, fail with a validation error that tells the user to run `toss account list` then `toss account use <accountSeq>`.

Rendering behavior:

- Text mode renders compact tables for common list responses.
- JSON mode returns the raw parsed `result` under `data` unless a command has documented metadata to include.
- Text rendering must not hide fields needed for financial decisions; if a response is too broad for a safe table, print key fields plus a hint to use `--json`.

## Phase 2: typed wrapper/library core

Add typed request/response models for the Phase 1 API surface and expose them from `toss-core`.

Rules:

- Prefer OpenAPI-derived field names and serde renames over hand-invented domain names.
- Use string/decimal-safe representations for prices, quantities, and money values. Do not use floating-point types for financial values.
- Keep endpoint wrappers thin: build query/body, call client, parse `result`.
- Keep generated or generated-like models isolated from handwritten client logic if code generation is used.

Tests:

- Config precedence tests.
- Token cache expiry tests.
- Request construction tests with a mock transport.
- Error envelope classification tests.
- CLI parser tests for each command group.
- Binary smoke tests for config and JSON envelope behavior.

## Phase 3: order-capable CLI

Order commands:

```bash
toss order buy --symbol <symbol> (--qty <qty> | --amount <amount>) --type <limit|market> [--price <price>]
toss order sell --symbol <symbol> --qty <qty> --type <limit|market> [--price <price>]
toss order modify <orderId> [--qty <qty>] [--price <price>]
toss order cancel <orderId>
toss order list --status <open|closed>
toss order show <orderId>
toss order buying-power --symbol <symbol>
toss order sellable-quantity --symbol <symbol>
toss order commissions --market <KR|US>
```

Safety rules:

- `--dry-run` must be available for every mutating order command.
- Mutating order commands must require an explicit confirmation flag or interactive confirmation.
- Large-order confirmation must expose Toss `confirmHighValueOrder` behavior explicitly.
- `clientOrderId` must be supported for create order idempotency.
- The CLI must not auto-generate `clientOrderId` unless the user opts in to that behavior.
- The output for dry-run must include method, path, account header presence, and request body with secrets omitted.

## Rate limiting

Implement a conservative client-side limiter keyed by Toss rate-limit group after the group mapping is extracted from OpenAPI descriptions.

Initial defaults from the overview:

- `AUTH`: 5 TPS
- `ACCOUNT`: 1 TPS
- `ASSET`: 5 TPS
- `STOCK`: 5 TPS
- `MARKET_INFO`: 3 TPS
- `MARKET_DATA`: 10 TPS
- `MARKET_DATA_CHART`: 5 TPS
- `ORDER`: 6 TPS, 3 TPS during 09:00-09:10 KST
- `ORDER_HISTORY`: 5 TPS
- `ORDER_INFO`: 6 TPS, 3 TPS during 09:00-09:10 KST

On HTTP 429, respect `Retry-After` when present and preserve `X-RateLimit-*` headers in the error metadata. Automatic retry is not part of Phase 1; it can be added after behavior is tested.

## Security

- Never print `client_secret`.
- Mask `client_id` in text config output except for the first and last few characters.
- Never include access tokens in normal command output.
- Token cache must be outside the repository.
- Order dry-run output must not include credentials or tokens.

## Verification checkpoints

Phase 1 is complete when:

- The workspace builds.
- Config loading works from file and environment.
- Token issuance request shape is tested without real credentials.
- Read-only commands parse and dispatch.
- JSON success/error envelopes are covered by tests.
- At least one read-only command can be smoke-tested with either real credentials or a mock transport.

Phase 2 is complete when:

- Phase 1 endpoint wrappers expose typed public APIs.
- Financial values avoid floating-point types.
- Mock transport tests cover request paths, headers, query parameters, and response parsing.

Phase 3 is complete when:

- Every mutating command has dry-run coverage.
- Confirmation behavior is tested.
- `clientOrderId` behavior is represented in request construction tests.
- Order API error classifications are tested for validation, conflict, insufficient buying power, market closed, and rate-limit cases.

## Open questions

- The public docs currently show a single production server; sandbox/staging support is not assumed.
- JWKS is mentioned in high-level orientation but not confirmed in the API reference index inspected for this design.
- The exact generated-vs-handwritten model strategy can be decided during implementation planning after checking Rust OpenAPI generator output quality for this spec.
