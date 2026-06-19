# Tossinvest CLI Safety Checklist

Use this checklist before credential checks, order feature work, or any live-order discussion.

## Credential Handling

- Keep real credentials outside the repository.
- Prefer `~/.config/tossinvest/config.yaml` with mode `0600`.
- Keep parent directory `~/.config/tossinvest` mode `0700`.
- Never commit local config, token cache, account snapshots, or command output containing private account details.
- Never print `client_secret` or access tokens.
- Treat `~/.tossinvest/token.json` as secret material.

## Read-only Verification

Safe commands:

```bash
toss --json config
toss --json auth token
toss --json account list
toss --json holdings
toss --json order buying-power --currency USD
toss --json order sellable-quantity --symbol AAPL
toss --json order commissions
toss --json order list --status open
toss --json order show <orderId>
```

Expected properties:

- `config` masks `client_id`.
- `auth token` proves OAuth without printing the token.
- Account-bound commands require valid `account_seq`.
- JSON output may contain private portfolio/account data; summarize carefully and avoid copying full private payloads into chat unless the user asks.

## Dry-run Verification

Safe dry-run examples:

```bash
toss --json order buy --symbol AAPL --qty 1 --type limit --price 1 --dry-run
toss --json order sell --symbol AAPL --qty 1 --type market --dry-run
toss --json order modify <orderId> --type limit --price 180 --dry-run
toss --json order cancel <orderId> --dry-run
```

Expected dry-run output:

```json
{
  "ok": true,
  "command": "order",
  "data": {
    "dryRun": true,
    "method": "POST",
    "path": "/api/v1/orders",
    "accountHeaderPresent": true,
    "body": {}
  }
}
```

Reject dry-run output if it includes:

- `Authorization`
- `Bearer`
- access token value
- `client_secret`
- raw `client_id`

## Live Order Gate

Do not run live order commands unless the user explicitly asks for a live order in the current conversation and provides:

- account/accountSeq to use
- side: buy or sell
- symbol
- exactly one size input: quantity or amount
- order type: limit or market
- price when required by order type
- `clientOrderId` decision for create orders
- high-value acknowledgement decision if relevant
- explicit `--confirm` intent

Even then, prefer this flow:

1. Run the equivalent `--dry-run` command.
2. Show the method/path/body only.
3. Ask the user to inspect the dry-run output.
4. Prefer asking the user to run the final live command locally.

## Order Contract

- `--dry-run` takes precedence over `--confirm`.
- `--confirm` gates live create/modify/cancel.
- `--confirm-high-value-order` maps to Toss `confirmHighValueOrder`; it is not a live-order confirmation substitute.
- `--client-order-id` maps to Toss `clientOrderId`; do not auto-generate it.
- Create order must include exactly one of `--qty` or `--amount`.
- `--amount` is for amount-based orders; Toss docs state it is US market and regular-hours constrained.

## Development Verification

Before claiming development work complete, run:

```bash
cargo fmt --all --manifest-path rust/Cargo.toml
cargo test --manifest-path rust/Cargo.toml
cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss
```

For install verification:

```bash
cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss --release
mkdir -p ~/.local/bin
install -m 755 rust/target/release/toss ~/.local/bin/toss
~/.local/bin/toss --json config
~/.local/bin/toss --json order buy --symbol AAPL --qty 1 --type limit --price 1 --dry-run
```
