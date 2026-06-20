# Toss CLI Safety Checklist

Use this checklist when operating the installed `toss` CLI, verifying credentials, reading account data, or discussing order commands. Keep the checklist focused on CLI usage and user safety, not Rust development.

## Credential Handling

- Keep real credentials outside repositories and chat transcripts.
- Prefer `~/.config/tossinvest/config.yaml` with mode `0600`.
- Keep parent directory `~/.config/tossinvest` mode `0700`.
- Never commit local config, token cache, account snapshots, or command output containing private account details.
- Never print `client_secret`, access tokens, refresh tokens, or token cache contents.
- Treat `~/.tossinvest/token.json` as secret material.

Safe config setup:

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

- `config` masks `client_id` and omits `client_secret`.
- `auth token` proves OAuth setup without printing the token.
- Account-bound commands require valid `account_seq` from config, environment, `--account`, or `toss account use`.
- JSON output may contain private portfolio/account data; summarize carefully and avoid copying full private payloads into chat unless the user asks.

## Account Selection

List accounts before choosing an account sequence:

```bash
toss account list
toss account use 1
```

Use `--account <seq>` for one-off overrides:

```bash
toss --account 1 holdings
toss --account 1 --json order buying-power --currency USD
```

Use `TOSSINVEST_ACCOUNT_SEQ` for a session-scoped default:

```bash
export TOSSINVEST_ACCOUNT_SEQ="1"
```

## Dry-run Verification

Safe dry-run examples:

```bash
toss --json order buy --symbol AAPL --qty 1 --type limit --price 1 --dry-run
toss --json order sell --symbol AAPL --qty 1 --type market --dry-run
toss --json order modify <orderId> --type limit --price 180 --dry-run
toss --json order cancel <orderId> --dry-run
```

Expected dry-run output shape:

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
- side: buy, sell, modify, or cancel
- symbol for create orders
- order ID for modify/cancel
- exactly one size input for create orders: quantity or amount
- order type: limit or market
- price when required by order type
- `clientOrderId` decision for create orders
- high-value acknowledgement decision if relevant
- explicit `--confirm` intent

Prefer this flow even after all details are present:

1. Run the equivalent `--dry-run` command.
2. Show method, path, account header presence, and body shape only.
3. Ask the user to inspect the dry-run output.
4. Prefer asking the user to run the final live command locally.

## Order Contract

- `--dry-run` takes precedence over `--confirm`.
- `--confirm` gates live buy/sell/modify/cancel commands.
- `--confirm-high-value-order` maps to Toss `confirmHighValueOrder`; it is not a live-order confirmation substitute.
- `--client-order-id` maps to Toss `clientOrderId`; do not auto-generate it.
- Create orders must include exactly one of `--qty` or `--amount`.
- `--amount` is for amount-based orders; Toss docs state it is US market and regular-hours constrained.
- No documented sandbox/staging support is assumed; live order traffic is treated as production.

## Safe Reporting

When reporting command results:

- State whether the command succeeded or failed.
- Name the output envelope shape for JSON commands.
- Avoid account numbers, balances, holdings, order IDs, and raw payloads unless the user explicitly asks for exact values.
- Never include token values, `client_secret`, or token cache contents.
