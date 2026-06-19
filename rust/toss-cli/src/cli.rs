use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Parser)]
#[command(name = "toss", about = "Toss Securities Open API CLI")]
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
    Auth(AuthArgs),
    Price(PriceArgs),
    Quote(QuoteArgs),
    Chart(ChartArgs),
    Stock(StockArgs),
    Market(MarketArgs),
    Account(AccountArgs),
    Holdings,
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Auth(_) => "auth",
            Self::Price(_) => "price",
            Self::Quote(_) => "quote",
            Self::Chart(_) => "chart",
            Self::Stock(_) => "stock",
            Self::Market(_) => "market",
            Self::Account(_) => "account",
            Self::Holdings => "holdings",
        }
    }
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
    pub symbol: String,
    #[arg(long, help = "comma-separated symbols; overrides positional symbol")]
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
    Trades(SymbolArg),
    Limits(SymbolArg),
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

#[derive(Debug, Args)]
pub struct CandlesArgs {
    pub symbol: String,
    #[arg(long)]
    pub interval: String,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
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
    ExchangeRate,
    Calendar(CalendarArgs),
}

#[derive(Debug, Args)]
pub struct CalendarArgs {
    #[command(subcommand)]
    pub command: CalendarCommand,
}

#[derive(Debug, Subcommand)]
pub enum CalendarCommand {
    Kr,
    Us,
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
    pub account_seq: u64,
}

#[derive(Debug, Args)]
pub struct SymbolArg {
    pub symbol: String,
}

#[derive(Debug, Args)]
pub struct SymbolsArg {
    #[arg(long)]
    pub symbols: String,
}
