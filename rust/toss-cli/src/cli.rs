use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

const QUICK_START: &str = r#"Quick Start:
  toss setup
  toss --json auth token
  toss account list
  toss account use 1
  toss price AAPL
  toss --json order buy --symbol AAPL --qty 1 --type limit --price 1 --dry-run

Safety:
  toss setup stores client_secret in the local config file as plaintext.
  Live order create/modify/cancel commands require --confirm.
"#;

#[derive(Debug, Parser)]
#[command(name = "toss", about = "Toss Securities Open API CLI", after_help = QUICK_START)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        help = "config file (default: ~/.config/tossinvest/config.yaml)"
    )]
    pub config: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        help = "accountSeq override for account-bound commands"
    )]
    pub account: Option<String>,
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Text)]
    pub output: OutputFormat,
    #[arg(long, global = true, help = "print successful command output as JSON")]
    pub json: bool,
    #[arg(long, global = true, help = "suppress extra text in text output")]
    pub quiet: bool,
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else {
            self.output
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Config,
    Setup(SetupArgs),
    Auth(AuthArgs),
    Price(PriceArgs),
    Quote(QuoteArgs),
    Chart(ChartArgs),
    Stock(StockArgs),
    Market(MarketArgs),
    Account(AccountArgs),
    Order(OrderArgs),
    Holdings(HoldingsArgs),
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Setup(_) => "setup",
            Self::Auth(_) => "auth",
            Self::Price(_) => "price",
            Self::Quote(_) => "quote",
            Self::Chart(_) => "chart",
            Self::Stock(_) => "stock",
            Self::Market(_) => "market",
            Self::Account(_) => "account",
            Self::Order(_) => "order",
            Self::Holdings(_) => "holdings",
        }
    }
}

#[derive(Debug, Args)]
pub struct OrderArgs {
    #[command(subcommand)]
    pub command: OrderCommand,
}

#[derive(Debug, Subcommand)]
pub enum OrderCommand {
    Buy(OrderCreateArgs),
    Sell(OrderCreateArgs),
    Modify(OrderModifyArgs),
    Cancel(OrderCancelArgs),
    List(OrderListArgs),
    Show(OrderShowArgs),
    BuyingPower(OrderBuyingPowerArgs),
    SellableQuantity(OrderSellableQuantityArgs),
    Commissions,
}

#[derive(Debug, Args)]
pub struct OrderCreateArgs {
    #[arg(long, value_parser = parse_symbol)]
    pub symbol: String,
    #[arg(long, value_parser = parse_integer_string)]
    pub qty: Option<String>,
    #[arg(long, value_parser = parse_decimal_string)]
    pub amount: Option<String>,
    #[arg(long = "type", value_enum)]
    pub order_type: OrderType,
    #[arg(long, value_parser = parse_decimal_string)]
    pub price: Option<String>,
    #[arg(long = "client-order-id", value_parser = parse_client_order_id)]
    pub client_order_id: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long = "confirm-high-value-order")]
    pub confirm_high_value_order: bool,
    #[arg(long = "time-in-force", value_enum)]
    pub time_in_force: Option<TimeInForceArg>,
}

#[derive(Debug, Args)]
pub struct OrderModifyArgs {
    pub order_id: String,
    #[arg(long, value_parser = parse_integer_string)]
    pub qty: Option<String>,
    #[arg(long = "type", value_enum)]
    pub order_type: OrderType,
    #[arg(long, value_parser = parse_decimal_string)]
    pub price: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long = "confirm-high-value-order")]
    pub confirm_high_value_order: bool,
}

#[derive(Debug, Args)]
pub struct OrderCancelArgs {
    pub order_id: String,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
}

#[derive(Debug, Args)]
pub struct OrderListArgs {
    #[arg(long, value_enum)]
    pub status: OrderHistoryStatus,
    #[arg(long, value_parser = parse_symbol)]
    pub symbol: Option<String>,
    #[arg(long, value_parser = parse_date)]
    pub from: Option<String>,
    #[arg(long, value_parser = parse_date)]
    pub to: Option<String>,
    #[arg(long)]
    pub cursor: Option<String>,
    #[arg(long, value_parser = clap::value_parser!(u16).range(1..=100))]
    pub limit: Option<u16>,
}

#[derive(Debug, Args)]
pub struct OrderShowArgs {
    pub order_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OrderHistoryStatus {
    Open,
    Closed,
}

impl std::fmt::Display for OrderHistoryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Open => "OPEN",
            Self::Closed => "CLOSED",
        })
    }
}

#[derive(Debug, Args)]
pub struct OrderBuyingPowerArgs {
    #[arg(long, value_enum)]
    pub currency: CurrencyArg,
}

#[derive(Debug, Args)]
pub struct OrderSellableQuantityArgs {
    #[arg(long, value_parser = parse_symbol)]
    pub symbol: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OrderType {
    Limit,
    Market,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Limit => "LIMIT",
            Self::Market => "MARKET",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CurrencyArg {
    #[value(name = "KRW", alias = "krw")]
    Krw,
    #[value(name = "USD", alias = "usd")]
    Usd,
}

impl std::fmt::Display for CurrencyArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Krw => "KRW",
            Self::Usd => "USD",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TimeInForceArg {
    Day,
    Cls,
}

impl std::fmt::Display for TimeInForceArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Day => "DAY",
            Self::Cls => "CLS",
        })
    }
}

#[derive(Debug, Args)]
pub struct SetupArgs {
    #[arg(long = "client-id", help = "client_id to save")]
    pub client_id: Option<String>,
    #[arg(
        long = "with-secret-stdin",
        help = "read client_secret from standard input instead of prompting"
    )]
    pub with_secret_stdin: bool,
    #[arg(long = "no-check", help = "skip token issuance check after saving")]
    pub no_check: bool,
}

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    Token,
}

#[derive(Debug, Args)]
pub struct PriceArgs {
    #[arg(value_parser = parse_symbol)]
    pub symbol: String,
    #[arg(long, help = "comma-separated symbols; overrides positional symbol", value_parser = parse_symbols)]
    pub symbols: Option<String>,
}

#[derive(Debug, Args)]
pub struct QuoteArgs {
    #[command(subcommand)]
    pub command: QuoteCommand,
}

#[derive(Debug, Subcommand)]
pub enum QuoteCommand {
    Orderbook(SymbolArg),
    Trades(TradesArgs),
    Limits(SymbolArg),
}

#[derive(Debug, Args)]
pub struct TradesArgs {
    #[arg(value_parser = parse_symbol)]
    pub symbol: String,
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=50))]
    pub count: Option<u8>,
}

#[derive(Debug, Args)]
pub struct ChartArgs {
    #[command(subcommand)]
    pub command: ChartCommand,
}

#[derive(Debug, Subcommand)]
pub enum ChartCommand {
    Candles(CandlesArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CandleInterval {
    #[value(name = "1m")]
    Min1,
    #[value(name = "1d")]
    Day1,
}

impl std::fmt::Display for CandleInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Min1 => "1m",
            Self::Day1 => "1d",
        })
    }
}

#[derive(Debug, Args)]
pub struct CandlesArgs {
    #[arg(value_parser = parse_symbol)]
    pub symbol: String,
    #[arg(long)]
    pub interval: CandleInterval,
    #[arg(long, value_parser = clap::value_parser!(u16).range(1..=200))]
    pub count: Option<u16>,
    #[arg(long, value_parser = parse_rfc3339)]
    pub before: Option<String>,
    #[arg(long)]
    pub adjusted: Option<bool>,
}

#[derive(Debug, Args)]
pub struct StockArgs {
    #[command(subcommand)]
    pub command: StockCommand,
}

#[derive(Debug, Subcommand)]
pub enum StockCommand {
    Get(SymbolArg),
    Warnings(SymbolArg),
    Search(SymbolsArg),
}

#[derive(Debug, Args)]
pub struct MarketArgs {
    #[command(subcommand)]
    pub command: MarketCommand,
}

#[derive(Debug, Subcommand)]
pub enum MarketCommand {
    ExchangeRate(ExchangeRateArgs),
    Calendar(CalendarArgs),
}

#[derive(Debug, Args)]
pub struct ExchangeRateArgs {
    #[arg(long, value_enum)]
    pub base: CurrencyArg,
    #[arg(long, value_enum)]
    pub quote: CurrencyArg,
    #[arg(long = "date-time", value_parser = parse_rfc3339)]
    pub date_time: Option<String>,
}

#[derive(Debug, Args)]
pub struct CalendarArgs {
    #[command(subcommand)]
    pub command: CalendarCommand,
}

#[derive(Debug, Subcommand)]
pub enum CalendarCommand {
    Kr(CalendarDateArgs),
    Us(CalendarDateArgs),
}

#[derive(Debug, Args)]
pub struct CalendarDateArgs {
    #[arg(long, value_parser = parse_date)]
    pub date: Option<String>,
}

#[derive(Debug, Args)]
pub struct HoldingsArgs {
    #[arg(long, value_parser = parse_symbol)]
    pub symbol: Option<String>,
}

#[derive(Debug, Args)]
pub struct AccountArgs {
    #[command(subcommand)]
    pub command: AccountCommand,
}

#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    List,
    Use(AccountUseArgs),
}

#[derive(Debug, Args)]
pub struct AccountUseArgs {
    #[arg(value_parser = clap::value_parser!(u64).range(0..=i64::MAX as u64))]
    pub account_seq: u64,
}

#[derive(Debug, Args)]
pub struct SymbolArg {
    #[arg(value_parser = parse_symbol)]
    pub symbol: String,
}

#[derive(Debug, Args)]
pub struct SymbolsArg {
    #[arg(long, value_parser = parse_symbols)]
    pub symbols: String,
}

fn parse_symbol(value: &str) -> Result<String, String> {
    if is_symbol(value) {
        Ok(value.to_string())
    } else {
        Err("symbol must contain only letters, digits, '.', or '-'".to_string())
    }
}

fn parse_symbols(value: &str) -> Result<String, String> {
    let count = value.split(',').count();
    if !(1..=200).contains(&count) {
        return Err("symbols must contain 1 to 200 comma-separated values".to_string());
    }
    if value.split(',').all(is_symbol) {
        Ok(value.to_string())
    } else {
        Err("symbols must contain only letters, digits, '.', '-', and commas".to_string())
    }
}

fn is_symbol(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
}

fn parse_date(value: &str) -> Result<String, String> {
    if chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok() {
        Ok(value.to_string())
    } else {
        Err("date must be YYYY-MM-DD".to_string())
    }
}

fn parse_rfc3339(value: &str) -> Result<String, String> {
    if chrono::DateTime::parse_from_rfc3339(value).is_ok() {
        Ok(value.to_string())
    } else {
        Err("date-time must be RFC3339, for example 2026-03-25T09:30:00+09:00".to_string())
    }
}

fn parse_client_order_id(value: &str) -> Result<String, String> {
    if !value.is_empty()
        && value.len() <= 36
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        Ok(value.to_string())
    } else {
        Err("client-order-id must be 1 to 36 letters, digits, '-', or '_'".to_string())
    }
}

fn parse_integer_string(value: &str) -> Result<String, String> {
    if !value.is_empty() && value.len() <= 30 && value.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(value.to_string())
    } else {
        Err("quantity must be a 1 to 30 digit integer string".to_string())
    }
}

fn parse_decimal_string(value: &str) -> Result<String, String> {
    let valid = if let Some((whole, fraction)) = value.split_once('.') {
        !whole.is_empty()
            && !fraction.is_empty()
            && whole.bytes().all(|byte| byte.is_ascii_digit())
            && fraction.bytes().all(|byte| byte.is_ascii_digit())
    } else {
        !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
    };
    if valid && value.len() <= 30 {
        Ok(value.to_string())
    } else {
        Err("decimal must be 1 to 30 digits with an optional fractional part".to_string())
    }
}
