use std::fs;
use std::io::Write;
use std::process::{Command as ProcessCommand, Stdio};

use clap::{Parser, error::ErrorKind};
use toss_cli::cli::{
    CalendarCommand, ChartCommand, Cli, Command, MarketCommand, OrderCommand, OrderHistoryStatus,
    OrderType, OutputFormat, QuoteCommand, StockCommand,
};

fn assert_json_parse_error(output: std::process::Output, command: &str) {
    assert!(!output.status.success());
    assert!(
        output.stderr.is_empty(),
        "{:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["command"], command);
    assert_eq!(envelope["error"]["kind"], "validation");
}

fn assert_json_runtime_validation_error(
    output: std::process::Output,
    command: &str,
) -> serde_json::Value {
    assert!(!output.status.success());
    assert!(
        output.stderr.is_empty(),
        "{:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["command"], command);
    assert_eq!(envelope["error"]["kind"], "validation");
    envelope
}

#[test]
fn prints_bb_style_version() {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_toss"))
        .arg("--version")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("toss version "), "{stdout}");
    assert!(stdout.contains("\ncommit: "), "{stdout}");
    assert!(stdout.contains("\nbuilt: "), "{stdout}");
}

#[test]
fn help_includes_quick_start() {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_toss"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Quick Start"), "{stdout}");
    assert!(stdout.contains("toss setup"), "{stdout}");
    assert!(stdout.contains("toss --json auth token"), "{stdout}");
    assert!(stdout.contains("toss --json order buy"), "{stdout}");
}

#[test]
fn parses_json_price_command() {
    let cli = Cli::parse_from(["toss", "--json", "price", "005930"]);
    assert_eq!(cli.output_format(), OutputFormat::Json);
    match cli.command {
        Command::Price(args) => assert_eq!(args.symbol, "005930"),
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_setup_command_without_secret_flag() {
    let cli = Cli::parse_from([
        "toss",
        "--json",
        "setup",
        "--client-id",
        "client-abc",
        "--with-secret-stdin",
        "--no-check",
        "--account",
        "7",
    ]);
    assert_eq!(cli.output_format(), OutputFormat::Json);
    assert_eq!(cli.account.as_deref(), Some("7"));
    match cli.command {
        Command::Setup(args) => {
            assert_eq!(args.client_id.as_deref(), Some("client-abc"));
            assert!(args.with_secret_stdin);
            assert!(args.no_check);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn rejects_invalid_chart_candle_interval() {
    let err =
        Cli::try_parse_from(["toss", "chart", "candles", "AAPL", "--interval", "2h"]).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidValue);
}

#[test]
fn rejects_chart_candle_count_over_openapi_limit() {
    let err = Cli::try_parse_from([
        "toss",
        "chart",
        "candles",
        "AAPL",
        "--interval",
        "1m",
        "--count",
        "201",
    ])
    .unwrap_err();

    assert_eq!(err.kind(), ErrorKind::ValueValidation);
}

#[test]
fn emits_json_validation_error_for_missing_price_symbol() {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_toss"))
        .args(["--json", "price"])
        .output()
        .unwrap();

    assert_json_parse_error(output, "price");
}

#[test]
fn emits_json_validation_error_for_missing_price_symbol_with_output_selector() {
    for args in [
        &["--output", "json", "price"][..],
        &["--output=json", "price"][..],
    ] {
        let output = ProcessCommand::new(env!("CARGO_BIN_EXE_toss"))
            .args(args)
            .output()
            .unwrap();

        assert_json_parse_error(output, "price");
    }
}

#[test]
fn emits_json_validation_error_for_invalid_account_override() {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_toss"))
        .args(["--output", "json", "--account", "abc", "config"])
        .output()
        .unwrap();

    assert_json_parse_error(output, "config");
}

#[test]
fn parses_quote_orderbook_command() {
    let cli = Cli::parse_from(["toss", "quote", "orderbook", "AAPL"]);
    match cli.command {
        Command::Quote(args) => match args.command {
            QuoteCommand::Orderbook(symbol) => assert_eq!(symbol.symbol, "AAPL"),
            other => panic!("unexpected quote command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_order_read_only_commands() {
    let cli = Cli::parse_from(["toss", "order", "buying-power", "--currency", "USD"]);
    match cli.command {
        Command::Order(args) => match args.command {
            OrderCommand::BuyingPower(args) => assert_eq!(args.currency, "USD"),
            other => panic!("unexpected order command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }

    let cli = Cli::parse_from(["toss", "order", "sellable-quantity", "--symbol", "AAPL"]);
    match cli.command {
        Command::Order(args) => match args.command {
            OrderCommand::SellableQuantity(args) => assert_eq!(args.symbol, "AAPL"),
            other => panic!("unexpected order command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }

    let cli = Cli::parse_from(["toss", "order", "commissions"]);
    match cli.command {
        Command::Order(args) => match args.command {
            OrderCommand::Commissions => {}
            other => panic!("unexpected order command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_order_history_commands() {
    let cli = Cli::parse_from(["toss", "order", "list", "--status", "open"]);
    match cli.command {
        Command::Order(args) => match args.command {
            OrderCommand::List(args) => assert_eq!(args.status, OrderHistoryStatus::Open),
            other => panic!("unexpected order command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }

    let cli = Cli::parse_from(["toss", "order", "show", "order-123"]);
    match cli.command {
        Command::Order(args) => match args.command {
            OrderCommand::Show(args) => assert_eq!(args.order_id, "order-123"),
            other => panic!("unexpected order command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }
}
#[test]
fn parses_order_mutating_commands_with_safety_flags() {
    let cli = Cli::parse_from([
        "toss",
        "order",
        "buy",
        "--symbol",
        "AAPL",
        "--qty",
        "1",
        "--type",
        "limit",
        "--price",
        "180",
        "--client-order-id",
        "client-1",
        "--confirm-high-value-order",
        "--dry-run",
        "--confirm",
    ]);
    match cli.command {
        Command::Order(args) => match args.command {
            OrderCommand::Buy(args) => {
                assert_eq!(args.symbol, "AAPL");
                assert_eq!(args.qty.as_deref(), Some("1"));
                assert_eq!(args.amount.as_deref(), None);
                assert_eq!(args.order_type, OrderType::Limit);
                assert_eq!(args.price.as_deref(), Some("180"));
                assert_eq!(args.client_order_id.as_deref(), Some("client-1"));
                assert!(args.dry_run);
                assert!(args.confirm);
                assert!(args.confirm_high_value_order);
            }
            other => panic!("unexpected order command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }

    let cli = Cli::parse_from([
        "toss",
        "order",
        "buy",
        "--symbol",
        "AAPL",
        "--amount",
        "100",
        "--type",
        "market",
        "--dry-run",
    ]);
    match cli.command {
        Command::Order(args) => match args.command {
            OrderCommand::Buy(args) => {
                assert_eq!(args.symbol, "AAPL");
                assert_eq!(args.qty.as_deref(), None);
                assert_eq!(args.amount.as_deref(), Some("100"));
                assert_eq!(args.order_type, OrderType::Market);
                assert_eq!(args.price.as_deref(), None);
                assert!(args.dry_run);
                assert!(!args.confirm);
                assert!(!args.confirm_high_value_order);
            }
            other => panic!("unexpected order command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }

    let cli = Cli::parse_from([
        "toss",
        "order",
        "sell",
        "--symbol",
        "AAPL",
        "--qty",
        "1",
        "--type",
        "market",
        "--dry-run",
    ]);
    match cli.command {
        Command::Order(args) => match args.command {
            OrderCommand::Sell(args) => {
                assert_eq!(args.symbol, "AAPL");
                assert_eq!(args.qty.as_deref(), Some("1"));
                assert_eq!(args.amount.as_deref(), None);
                assert_eq!(args.order_type, OrderType::Market);
                assert!(args.dry_run);
                assert!(!args.confirm);
                assert!(!args.confirm_high_value_order);
            }
            other => panic!("unexpected order command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }

    let cli = Cli::parse_from([
        "toss",
        "order",
        "modify",
        "opaque-id",
        "--type",
        "limit",
        "--price",
        "180",
        "--confirm-high-value-order",
        "--dry-run",
    ]);
    match cli.command {
        Command::Order(args) => match args.command {
            OrderCommand::Modify(args) => {
                assert_eq!(args.order_id, "opaque-id");
                assert_eq!(args.qty.as_deref(), None);
                assert_eq!(args.order_type, OrderType::Limit);
                assert_eq!(args.price.as_deref(), Some("180"));
                assert!(args.dry_run);
                assert!(!args.confirm);
                assert!(args.confirm_high_value_order);
            }
            other => panic!("unexpected order command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }

    let cli = Cli::parse_from([
        "toss",
        "order",
        "cancel",
        "opaque-id",
        "--dry-run",
        "--confirm",
    ]);
    match cli.command {
        Command::Order(args) => match args.command {
            OrderCommand::Cancel(args) => {
                assert_eq!(args.order_id, "opaque-id");
                assert!(args.dry_run);
                assert!(args.confirm);
            }
            other => panic!("unexpected order command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn rejects_order_flags_outside_mutating_commands() {
    let err = Cli::try_parse_from([
        "toss",
        "order",
        "buying-power",
        "--currency",
        "USD",
        "--dry-run",
    ])
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::UnknownArgument);

    let err = Cli::try_parse_from([
        "toss",
        "order",
        "sellable-quantity",
        "--symbol",
        "AAPL",
        "--confirm",
    ])
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::UnknownArgument);

    let err = Cli::try_parse_from([
        "toss",
        "order",
        "modify",
        "opaque-id",
        "--type",
        "limit",
        "--client-order-id",
        "client-1",
    ])
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::UnknownArgument);
}

#[test]
fn runs_config_command_through_binary() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    fs::write(
        &config,
        "client_id: client-abc\nclient_secret: secret-xyz\naccount_seq: 5\n",
    )
    .unwrap();

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_toss"))
        .args(["--config", config.to_str().unwrap(), "--json", "config"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["command"], "config");
    assert_eq!(envelope["data"]["client_id"], "clie****-abc");
    assert_eq!(envelope["data"]["account_seq"], 5);
    assert!(envelope["data"].get("client_secret").is_none());
}

#[test]
fn runs_account_use_command_through_binary() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    fs::write(
        &config,
        "client_id: client-abc\nclient_secret: secret-xyz\n",
    )
    .unwrap();

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_toss"))
        .args([
            "--config",
            config.to_str().unwrap(),
            "--json",
            "account",
            "use",
            "42",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"ok\":true"));
    assert!(stdout.contains("\"account_seq\":42"));
    assert!(stdout.contains("\"config_path\":"));

    let contents = fs::read_to_string(&config).unwrap();
    assert!(contents.contains("account_seq: 42"));
    assert!(contents.contains("client_id: client-abc"));
    assert!(contents.contains("client_secret: secret-xyz"));
}

#[test]
fn runs_setup_command_through_binary_without_printing_secret() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");

    let mut child = ProcessCommand::new(env!("CARGO_BIN_EXE_toss"))
        .args([
            "--config",
            config.to_str().unwrap(),
            "--json",
            "setup",
            "--client-id",
            "client-abc",
            "--with-secret-stdin",
            "--no-check",
            "--account",
            "7",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"secret-xyz\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("secret-xyz"), "{stdout}");
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["command"], "setup");
    assert_eq!(envelope["data"]["credentials"], "configured");
    assert_eq!(envelope["data"]["token_check"], "skipped");
    assert_eq!(envelope["data"]["account_seq"], 7);
    assert!(envelope["data"].get("client_secret").is_none());

    let contents = fs::read_to_string(&config).unwrap();
    assert!(contents.contains("client_id: client-abc"));
    assert!(contents.contains("client_secret: secret-xyz"));
    assert!(contents.contains("account_seq: 7"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = fs::metadata(&config).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode & 0o077, 0, "config must not be group/world readable");
    }
}

#[test]
fn runs_order_buy_dry_run_command_through_binary() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    fs::write(
        &config,
        "client_id: client-abc\nclient_secret: secret-xyz\naccount_seq: 5\n",
    )
    .unwrap();

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_toss"))
        .args([
            "--config",
            config.to_str().unwrap(),
            "--json",
            "order",
            "buy",
            "--symbol",
            "AAPL",
            "--qty",
            "1",
            "--type",
            "limit",
            "--price",
            "180",
            "--client-order-id",
            "client-1",
            "--confirm-high-value-order",
            "--dry-run",
            "--confirm",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["command"], "order");
    assert_eq!(envelope["data"]["dryRun"], true);
    assert_eq!(envelope["data"]["method"], "POST");
    assert_eq!(envelope["data"]["path"], "/api/v1/orders");
    assert_eq!(envelope["data"]["accountHeaderPresent"], true);
    assert_eq!(
        envelope["data"]["body"],
        serde_json::json!({
            "clientOrderId": "client-1",
            "symbol": "AAPL",
            "side": "BUY",
            "orderType": "LIMIT",
            "quantity": "1",
            "price": "180",
            "confirmHighValueOrder": true
        })
    );
    assert!(
        envelope["data"].get("authorization").is_none(),
        "{envelope}"
    );
    assert!(
        envelope["data"].get("client_secret").is_none(),
        "{envelope}"
    );
    assert!(envelope["data"].get("client_id").is_none(), "{envelope}");
}

#[test]
fn emits_json_validation_error_for_order_buy_without_dry_run_or_confirm() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    fs::write(
        &config,
        "client_id: client-abc\nclient_secret: secret-xyz\naccount_seq: 5\n",
    )
    .unwrap();

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_toss"))
        .args([
            "--config",
            config.to_str().unwrap(),
            "--json",
            "order",
            "buy",
            "--symbol",
            "AAPL",
            "--qty",
            "1",
            "--type",
            "limit",
            "--price",
            "180",
        ])
        .output()
        .unwrap();

    let envelope = assert_json_runtime_validation_error(output, "order");
    let message = envelope["error"]["message"].as_str().unwrap();
    assert!(message.contains("--dry-run"), "{message}");
    assert!(message.contains("--confirm"), "{message}");
}

#[test]
fn parses_chart_candles_command() {
    let cli = Cli::parse_from([
        "toss",
        "chart",
        "candles",
        "AAPL",
        "--interval",
        "1d",
        "--count",
        "200",
        "--before",
        "2026-06-19T18:20:00+09:00",
        "--adjusted",
        "false",
    ]);
    match cli.command {
        Command::Chart(args) => match args.command {
            ChartCommand::Candles(args) => {
                assert_eq!(args.symbol, "AAPL");
                assert_eq!(args.interval.to_string(), "1d");
                assert_eq!(args.count, Some(200));
                assert_eq!(args.before.as_deref(), Some("2026-06-19T18:20:00+09:00"));
                assert_eq!(args.adjusted, Some(false));
            }
        },
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_stock_and_market_commands() {
    let stock = Cli::parse_from(["toss", "stock", "search", "--symbols", "005930,AAPL"]);
    match stock.command {
        Command::Stock(args) => match args.command {
            StockCommand::Search(args) => assert_eq!(args.symbols, "005930,AAPL"),
            other => panic!("unexpected stock command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }

    let market = Cli::parse_from(["toss", "market", "calendar", "kr"]);
    match market.command {
        Command::Market(args) => match args.command {
            MarketCommand::Calendar(args) => match args.command {
                CalendarCommand::Kr => {}
                other => panic!("unexpected calendar command: {other:?}"),
            },
            other => panic!("unexpected market command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }
}
