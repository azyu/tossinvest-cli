use std::fs;
use std::process::Command as ProcessCommand;

use clap::Parser;
use toss_cli::cli::{Cli, Command, OutputFormat, QuoteCommand};

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
    fs::write(&config, "client_id: client-abc\nclient_secret: secret-xyz\naccount_seq: 5\n")
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
    fs::write(&config, "client_id: client-abc\nclient_secret: secret-xyz\n").unwrap();

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
