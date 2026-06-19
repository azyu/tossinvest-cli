use std::fs;
use std::process::Command as ProcessCommand;

use clap::{Parser, error::ErrorKind};
use toss_cli::cli::{
    CalendarCommand, ChartCommand, Cli, Command, MarketCommand, OutputFormat, QuoteCommand,
    StockCommand,
};

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
fn rejects_invalid_chart_candle_interval() {
    let err =
        Cli::try_parse_from(["toss", "chart", "candles", "AAPL", "--interval", "2h"]).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidValue);
}

#[test]
fn emits_json_validation_error_for_missing_price_symbol() {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_toss"))
        .args(["--json", "price"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        output.stderr.is_empty(),
        "{:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["command"], "price");
    assert_eq!(envelope["error"]["kind"], "validation");
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
    assert!(stdout.contains("\"ok\":true"));
    assert!(stdout.contains("\"account_seq\":5"));
    assert!(!stdout.contains("secret-xyz"));
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
fn parses_chart_candles_command() {
    let cli = Cli::parse_from([
        "toss",
        "chart",
        "candles",
        "AAPL",
        "--interval",
        "1d",
        "--from",
        "2026-01-01",
    ]);
    match cli.command {
        Command::Chart(args) => match args.command {
            ChartCommand::Candles(args) => {
                assert_eq!(args.symbol, "AAPL");
                assert_eq!(args.interval.to_string(), "1d");
                assert_eq!(args.from.as_deref(), Some("2026-01-01"));
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
