use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text output.
    Text,
    /// Stable JSON envelope output for automation.
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
    #[arg(
        long,
        global = true,
        value_enum,
        default_value_t = OutputFormat::Text,
        help = "Select text or JSON output"
    )]
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
    /// Show resolved configuration without printing secrets.
    Config,
    /// Verify authentication flows without printing tokens.
    Auth(AuthArgs),
    /// Current price for one or more stock symbols.
    Price(PriceArgs),
    /// Quote data such as orderbooks, trades, and limits.
    Quote(QuoteArgs),
    /// Historical candle chart data.
    Chart(ChartArgs),
    /// Stock metadata, warning, and search commands.
    Stock(StockArgs),
    /// Market exchange-rate and calendar commands.
    Market(MarketArgs),
    /// List accounts or persist a default account sequence.
    Account(AccountArgs),
    /// Read order info and perform gated order actions.
    Order(OrderArgs),
    /// Account holdings for the selected account.
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
            Self::Order(_) => "order",
            Self::Holdings => "holdings",
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
    /// Create a gated buy order or print a dry-run request shape.
    Buy(OrderCreateArgs),
    /// Create a gated sell order or print a dry-run request shape.
    Sell(OrderCreateArgs),
    /// Modify an existing order behind dry-run/confirm safety gates.
    Modify(OrderModifyArgs),
    /// Cancel an existing order behind dry-run/confirm safety gates.
    Cancel(OrderCancelArgs),
    /// List open or closed orders for the selected account.
    List(OrderListArgs),
    /// Show one order by order ID.
    Show(OrderShowArgs),
    /// Query buying power by currency.
    BuyingPower(OrderBuyingPowerArgs),
    /// Query sellable quantity for a symbol.
    SellableQuantity(OrderSellableQuantityArgs),
    /// Query commission rates and settings.
    Commissions,
}

#[derive(Debug, Args)]
pub struct OrderCreateArgs {
    #[arg(long, help = "Stock symbol to buy or sell")]
    pub symbol: String,
    #[arg(long, help = "Share quantity; mutually exclusive with --amount")]
    pub qty: Option<String>,
    #[arg(
        long,
        help = "Cash amount for amount-based orders; mutually exclusive with --qty"
    )]
    pub amount: Option<String>,
    #[arg(long = "type", value_enum, help = "Order type")]
    pub order_type: OrderType,
    #[arg(long, help = "Limit price when required by the order type")]
    pub price: Option<String>,
    #[arg(long = "client-order-id", help = "Client-supplied idempotency key")]
    pub client_order_id: Option<String>,
    #[arg(long, help = "Do not submit; print the request shape only")]
    pub dry_run: bool,
    #[arg(long, help = "Explicitly allow a live brokerage order")]
    pub confirm: bool,
    #[arg(
        long = "confirm-high-value-order",
        help = "Acknowledge Toss high-value order confirmation when applicable"
    )]
    pub confirm_high_value_order: bool,
}

#[derive(Debug, Args)]
pub struct OrderModifyArgs {
    #[arg(help = "Order ID to modify")]
    pub order_id: String,
    #[arg(long, help = "New share quantity")]
    pub qty: Option<String>,
    #[arg(long = "type", value_enum, help = "New order type")]
    pub order_type: OrderType,
    #[arg(long, help = "New limit price when required by the order type")]
    pub price: Option<String>,
    #[arg(long, help = "Do not submit; print the request shape only")]
    pub dry_run: bool,
    #[arg(long, help = "Explicitly allow a live brokerage order")]
    pub confirm: bool,
    #[arg(
        long = "confirm-high-value-order",
        help = "Acknowledge Toss high-value order confirmation when applicable"
    )]
    pub confirm_high_value_order: bool,
}

#[derive(Debug, Args)]
pub struct OrderCancelArgs {
    #[arg(help = "Order ID to cancel")]
    pub order_id: String,
    #[arg(long, help = "Do not submit; print the request shape only")]
    pub dry_run: bool,
    #[arg(long, help = "Explicitly allow a live brokerage order")]
    pub confirm: bool,
}

#[derive(Debug, Args)]
pub struct OrderListArgs {
    #[arg(long, value_enum, help = "Order status to list")]
    pub status: OrderHistoryStatus,
}

#[derive(Debug, Args)]
pub struct OrderShowArgs {
    #[arg(help = "Order ID to show")]
    pub order_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OrderHistoryStatus {
    /// Open orders.
    Open,
    /// Closed orders.
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
    #[arg(long, help = "Currency code, such as USD or KRW")]
    pub currency: String,
}

#[derive(Debug, Args)]
pub struct OrderSellableQuantityArgs {
    #[arg(long, help = "Stock symbol to check")]
    pub symbol: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OrderType {
    /// Limit order.
    Limit,
    /// Market order.
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
    /// Buy side.
    Buy,
    /// Sell side.
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

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Request and validate an OAuth token without printing it.
    Token,
}

#[derive(Debug, Args)]
pub struct PriceArgs {
    #[arg(help = "Stock symbol to query")]
    pub symbol: String,
    #[arg(long, help = "Comma-separated symbols; overrides positional symbol")]
    pub symbols: Option<String>,
}

#[derive(Debug, Args)]
pub struct QuoteArgs {
    #[command(subcommand)]
    pub command: QuoteCommand,
}

#[derive(Debug, Subcommand)]
pub enum QuoteCommand {
    /// Current orderbook for a symbol.
    Orderbook(SymbolArg),
    /// Recent trades for a symbol.
    Trades(SymbolArg),
    /// Price limits for a symbol.
    Limits(SymbolArg),
}

#[derive(Debug, Args)]
pub struct ChartArgs {
    #[command(subcommand)]
    pub command: ChartCommand,
}

#[derive(Debug, Subcommand)]
pub enum ChartCommand {
    /// Candle data for a symbol and interval.
    Candles(CandlesArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CandleInterval {
    /// One-minute candles.
    #[value(name = "1m")]
    Min1,
    /// Daily candles.
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
    #[arg(help = "Stock symbol to query")]
    pub symbol: String,
    #[arg(long, help = "Candle interval")]
    pub interval: CandleInterval,
    #[arg(long, help = "Optional start timestamp/date accepted by Toss API")]
    pub from: Option<String>,
    #[arg(long, help = "Optional end timestamp/date accepted by Toss API")]
    pub to: Option<String>,
}

#[derive(Debug, Args)]
pub struct StockArgs {
    #[command(subcommand)]
    pub command: StockCommand,
}

#[derive(Debug, Subcommand)]
pub enum StockCommand {
    /// Stock metadata for one symbol.
    Get(SymbolArg),
    /// Warning information for one symbol.
    Warnings(SymbolArg),
    /// Search multiple symbols.
    Search(SymbolsArg),
}

#[derive(Debug, Args)]
pub struct MarketArgs {
    #[command(subcommand)]
    pub command: MarketCommand,
}

#[derive(Debug, Subcommand)]
pub enum MarketCommand {
    /// Current exchange-rate information.
    ExchangeRate,
    /// Market calendar by region.
    Calendar(CalendarArgs),
}

#[derive(Debug, Args)]
pub struct CalendarArgs {
    #[command(subcommand)]
    pub command: CalendarCommand,
}

#[derive(Debug, Subcommand)]
pub enum CalendarCommand {
    /// Korean market calendar.
    Kr,
    /// US market calendar.
    Us,
}

#[derive(Debug, Args)]
pub struct AccountArgs {
    #[command(subcommand)]
    pub command: AccountCommand,
}

#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    /// List accounts available to the API credentials.
    List,
    /// Persist a default account sequence to the local config.
    Use(AccountUseArgs),
}

#[derive(Debug, Args)]
pub struct AccountUseArgs {
    #[arg(help = "Account sequence to persist")]
    pub account_seq: u64,
}

#[derive(Debug, Args)]
pub struct SymbolArg {
    #[arg(help = "Stock symbol to query")]
    pub symbol: String,
}

#[derive(Debug, Args)]
pub struct SymbolsArg {
    #[arg(long, help = "Comma-separated stock symbols")]
    pub symbols: String,
}
