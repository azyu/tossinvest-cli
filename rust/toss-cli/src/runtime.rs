use std::io::{self, Write};

use anyhow::Result;
use serde::Serialize;
use serde_json::{Value, json};
use toss_core::TossError;
use toss_core::account;
use toss_core::asset;
use toss_core::client::TossClient;
use toss_core::config::{self, AppConfig};
use toss_core::market_data;
use toss_core::market_info;
use toss_core::order;
use toss_core::order_info;
use toss_core::stock_info;
use toss_core::transport::{HttpMethod, HttpRequest, Transport};
use toss_core::{OrderCreateRequest, OrderModifyRequest, OrderSide as CoreOrderSide};

use crate::cli::{self, OutputFormat};
use crate::render;
#[derive(Debug, Serialize)]
struct SuccessEnvelope<'a, T> {
    ok: bool,
    command: &'a str,
    data: T,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope<'a> {
    ok: bool,
    command: &'a str,
    error: ErrorOutput,
}

#[derive(Debug, Serialize)]
struct ErrorOutput {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    message: String,
    #[serde(rename = "requestId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
}

pub async fn run(cli: cli::Cli, writer: &mut dyn Write) -> Result<()> {
    let cli::Cli {
        config,
        account,
        output,
        json,
        quiet: _,
        command,
    } = cli;
    let command_name = command.name();
    let output_format = if json { OutputFormat::Json } else { output };
    if let cli::Command::Setup(args) = command {
        return run_setup(
            output_format,
            command_name,
            config.as_deref(),
            account.as_deref(),
            args,
            writer,
        )
        .await;
    }

    let app_config = config::load(config.as_deref(), account.as_deref())?;
    match command {
        cli::Command::Config => run_config(output_format, command_name, &app_config, writer),
        cli::Command::Account(cli::AccountArgs {
            command: cli::AccountCommand::Use(args),
        }) => {
            let path = config::save_account_seq(config.as_deref(), args.account_seq)?;
            write_output(
                output_format,
                command_name,
                json!({ "config_path": path, "account_seq": args.account_seq }),
                writer,
            )
        }
        command => {
            let client = TossClient::new(app_config)?;
            run_client_command(output_format, command_name, command, &client, writer).await
        }
    }
}

async fn run_setup(
    output_format: OutputFormat,
    command: &str,
    config_path: Option<&std::path::Path>,
    account_override: Option<&str>,
    args: cli::SetupArgs,
    writer: &mut dyn Write,
) -> Result<()> {
    let client_id = match args.client_id {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => prompt_line("client_id: ")?,
    };
    if client_id.is_empty() {
        return Err(TossError::Validation("client_id is required".to_string()).into());
    }

    let client_secret = if args.with_secret_stdin {
        read_secret_from_stdin()?
    } else {
        rpassword::prompt_password("client_secret: ")?
            .trim()
            .to_string()
    };
    if client_secret.is_empty() {
        return Err(TossError::Validation("client_secret is required".to_string()).into());
    }

    let account_seq = account_override
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| TossError::Validation(format!("invalid account sequence: {value}")))
        })
        .transpose()?;
    let saved_path = config::save_config(
        config_path,
        config::ConfigUpdate {
            client_id: Some(client_id),
            client_secret: Some(client_secret),
            account_seq: account_seq.map(Some),
        },
    )?;

    let token_check = if args.no_check {
        "skipped"
    } else {
        let app_config = config::load(config_path, None)?;
        let client = TossClient::new(app_config)?;
        client.check_token().await?;
        "ok"
    };

    write_output(
        output_format,
        command,
        json!({
            "config_path": saved_path,
            "credentials": "configured",
            "token_check": token_check,
            "account_seq": account_seq,
        }),
        writer,
    )
}

fn prompt_line(prompt: &str) -> Result<String> {
    eprint!("{prompt}");
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_string())
}

fn read_secret_from_stdin() -> Result<String> {
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_string())
}

async fn run_client_command<T: Transport>(
    output_format: OutputFormat,
    command_name: &str,
    command: cli::Command,
    client: &TossClient<T>,
    writer: &mut dyn Write,
) -> Result<()> {
    let value = match command {
        cli::Command::Auth(args) => match args.command {
            cli::AuthCommand::Token => {
                client.check_token().await?;
                json!({ "token_check": "ok" })
            }
        },
        cli::Command::Price(args) => {
            let symbols = args.symbols.as_deref().unwrap_or(&args.symbol);
            serde_json::to_value(market_data::prices(client, symbols).await?)?
        }
        cli::Command::Quote(args) => match args.command {
            cli::QuoteCommand::Orderbook(arg) => {
                serde_json::to_value(market_data::orderbook(client, &arg.symbol).await?)?
            }
            cli::QuoteCommand::Trades(arg) => {
                serde_json::to_value(market_data::trades(client, &arg.symbol).await?)?
            }
            cli::QuoteCommand::Limits(arg) => {
                serde_json::to_value(market_data::price_limits(client, &arg.symbol).await?)?
            }
        },
        cli::Command::Chart(args) => match args.command {
            cli::ChartCommand::Candles(args) => {
                let mut query = vec![
                    ("symbol".to_string(), args.symbol),
                    ("interval".to_string(), args.interval.to_string()),
                ];
                if let Some(count) = args.count {
                    query.push(("count".to_string(), count.to_string()));
                }
                if let Some(before) = args.before {
                    query.push(("before".to_string(), before));
                }
                if let Some(adjusted) = args.adjusted {
                    query.push(("adjusted".to_string(), adjusted.to_string()));
                }
                serde_json::to_value(market_data::candles(client, query).await?)?
            }
        },
        cli::Command::Stock(args) => match args.command {
            cli::StockCommand::Get(arg) => {
                serde_json::to_value(stock_info::stocks(client, &arg.symbol).await?)?
            }
            cli::StockCommand::Warnings(arg) => {
                serde_json::to_value(stock_info::warnings(client, &arg.symbol).await?)?
            }
            cli::StockCommand::Search(arg) => {
                serde_json::to_value(stock_info::stocks(client, &arg.symbols).await?)?
            }
        },
        cli::Command::Market(args) => match args.command {
            cli::MarketCommand::ExchangeRate => {
                serde_json::to_value(market_info::exchange_rate(client).await?)?
            }
            cli::MarketCommand::Calendar(args) => match args.command {
                cli::CalendarCommand::Kr => {
                    serde_json::to_value(market_info::kr_calendar(client).await?)?
                }
                cli::CalendarCommand::Us => {
                    serde_json::to_value(market_info::us_calendar(client).await?)?
                }
            },
        },
        cli::Command::Account(args) => match args.command {
            cli::AccountCommand::List => serde_json::to_value(account::list(client).await?)?,
            cli::AccountCommand::Use(_) => {
                unreachable!("account use is handled before client dispatch")
            }
        },
        cli::Command::Order(args) => match args.command {
            cli::OrderCommand::Buy(args) => {
                validate_order_create_size(&args, "buy")?;
                if args.dry_run {
                    let request = order::build_create_dry_run(
                        client,
                        &build_order_create_request(&args, CoreOrderSide::BUY),
                    )
                    .await?;
                    dry_run_output(request)?
                } else if args.confirm {
                    serde_json::to_value(
                        order::create(
                            client,
                            &build_order_create_request(&args, CoreOrderSide::BUY),
                        )
                        .await?,
                    )?
                } else {
                    return Err(toss_core::TossError::Validation(
                        "live order commands require --confirm or use --dry-run".to_string(),
                    )
                    .into());
                }
            }
            cli::OrderCommand::Sell(args) => {
                validate_order_create_size(&args, "sell")?;
                if args.dry_run {
                    let request = order::build_create_dry_run(
                        client,
                        &build_order_create_request(&args, CoreOrderSide::SELL),
                    )
                    .await?;
                    dry_run_output(request)?
                } else if args.confirm {
                    serde_json::to_value(
                        order::create(
                            client,
                            &build_order_create_request(&args, CoreOrderSide::SELL),
                        )
                        .await?,
                    )?
                } else {
                    return Err(toss_core::TossError::Validation(
                        "live order commands require --confirm or use --dry-run".to_string(),
                    )
                    .into());
                }
            }
            cli::OrderCommand::Modify(args) => {
                if args.dry_run {
                    let request = order::build_modify_dry_run(
                        client,
                        &args.order_id,
                        &build_order_modify_request(&args),
                    )
                    .await?;
                    dry_run_output(request)?
                } else if args.confirm {
                    serde_json::to_value(
                        order::modify(client, &args.order_id, &build_order_modify_request(&args))
                            .await?,
                    )?
                } else {
                    return Err(toss_core::TossError::Validation(
                        "live order commands require --confirm or use --dry-run".to_string(),
                    )
                    .into());
                }
            }
            cli::OrderCommand::Cancel(args) => {
                if args.dry_run {
                    let request = order::build_cancel_dry_run(client, &args.order_id).await?;
                    dry_run_output(request)?
                } else if args.confirm {
                    serde_json::to_value(order::cancel(client, &args.order_id).await?)?
                } else {
                    return Err(toss_core::TossError::Validation(
                        "live order commands require --confirm or use --dry-run".to_string(),
                    )
                    .into());
                }
            }
            cli::OrderCommand::List(args) => {
                let status = match args.status {
                    cli::OrderHistoryStatus::Open => "OPEN",
                    cli::OrderHistoryStatus::Closed => "CLOSED",
                };
                serde_json::to_value(order::list(client, status).await?)?
            }
            cli::OrderCommand::Show(args) => {
                serde_json::to_value(order::show(client, &args.order_id).await?)?
            }
            cli::OrderCommand::BuyingPower(args) => {
                serde_json::to_value(order_info::buying_power(client, &args.currency).await?)?
            }
            cli::OrderCommand::SellableQuantity(args) => {
                serde_json::to_value(order_info::sellable_quantity(client, &args.symbol).await?)?
            }
            cli::OrderCommand::Commissions => {
                serde_json::to_value(order_info::commissions(client).await?)?
            }
        },
        cli::Command::Holdings => serde_json::to_value(asset::holdings(client).await?)?,
        cli::Command::Config => unreachable!("config is handled before client dispatch"),
        cli::Command::Setup(_) => unreachable!("setup is handled before client dispatch"),
    };
    write_output(output_format, command_name, value, writer)
}

fn build_order_create_request(
    args: &cli::OrderCreateArgs,
    side: CoreOrderSide,
) -> OrderCreateRequest {
    let order_type = match args.order_type {
        cli::OrderType::Limit => toss_core::OrderType::LIMIT,
        cli::OrderType::Market => toss_core::OrderType::MARKET,
    };
    OrderCreateRequest {
        client_order_id: args.client_order_id.clone(),
        symbol: args.symbol.clone(),
        side,
        order_type,
        time_in_force: None,
        quantity: args.qty.as_ref().map(|value| json!(value)),
        price: args.price.as_ref().map(|value| json!(value)),
        confirm_high_value_order: args.confirm_high_value_order.then_some(true),
        order_amount: args.amount.as_ref().map(|value| json!(value)),
    }
}

fn build_order_modify_request(args: &cli::OrderModifyArgs) -> OrderModifyRequest {
    let order_type = match args.order_type {
        cli::OrderType::Limit => toss_core::OrderType::LIMIT,
        cli::OrderType::Market => toss_core::OrderType::MARKET,
    };
    OrderModifyRequest {
        order_type,
        quantity: args.qty.as_ref().map(|value| json!(value)),
        price: args.price.as_ref().map(|value| json!(value)),
        confirm_high_value_order: args.confirm_high_value_order.then_some(true),
    }
}

fn dry_run_output(request: HttpRequest) -> Result<Value> {
    let HttpRequest {
        method,
        path,
        headers,

        body,
        ..
    } = request;
    let body = match body {
        Some(body) => serde_json::from_slice(&body)?,
        None => Value::Null,
    };
    Ok(json!({
        "dryRun": true,
        "method": http_method_name(method),
        "path": path,
        "accountHeaderPresent": headers
            .iter()
            .any(|header| header.name == "X-Tossinvest-Account"),
        "body": body,
    }))
}

fn validate_order_create_size(args: &cli::OrderCreateArgs, side: &str) -> Result<()> {
    let has_qty = args.qty.is_some();
    let has_amount = args.amount.is_some();
    if has_qty == has_amount {
        return Err(TossError::Validation(format!(
            "order {side} requires exactly one of --qty or --amount"
        ))
        .into());
    }
    Ok(())
}

fn http_method_name(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
    }
}

fn write_output(
    output_format: OutputFormat,
    command: &str,
    data: Value,
    writer: &mut dyn Write,
) -> Result<()> {
    match output_format {
        OutputFormat::Json => {
            serde_json::to_writer(
                &mut *writer,
                &SuccessEnvelope {
                    ok: true,
                    command,
                    data,
                },
            )?;
            writeln!(&mut *writer)?;
        }
        OutputFormat::Text => {
            let value = data;
            if command == "config" {
                render::write_key_values(
                    writer,
                    &[
                        (
                            "client_id",
                            value["client_id"].as_str().unwrap_or("-").to_string(),
                        ),
                        (
                            "account_seq",
                            value["account_seq"]
                                .as_u64()
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "-".to_string()),
                        ),
                    ],
                )?;
            } else {
                serde_json::to_writer_pretty(&mut *writer, &value)?;
                writeln!(&mut *writer)?;
            }
        }
    }
    Ok(())
}

fn run_config(
    output_format: OutputFormat,
    command: &str,
    app_config: &AppConfig,
    writer: &mut dyn Write,
) -> Result<()> {
    let data = json!({
        "client_id": mask_client_id(&app_config.client_id),
        "account_seq": app_config.account_seq,
    });
    write_output(output_format, command, data, writer)
}

pub fn write_json_error(writer: &mut dyn Write, command: &str, err: &anyhow::Error) -> Result<()> {
    let error = classify_error(err);
    serde_json::to_writer(
        &mut *writer,
        &ErrorEnvelope {
            ok: false,
            command,
            error,
        },
    )?;
    writeln!(&mut *writer)?;
    Ok(())
}

fn classify_error(err: &anyhow::Error) -> ErrorOutput {
    if let Some(toss) = err.downcast_ref::<TossError>() {
        match toss {
            TossError::Config(message) => {
                return ErrorOutput {
                    kind: "config",
                    code: None,
                    message: message.clone(),
                    request_id: None,
                };
            }
            TossError::Validation(message) => {
                return ErrorOutput {
                    kind: "validation",
                    code: None,
                    message: message.clone(),
                    request_id: None,
                };
            }
            TossError::Auth(message) => {
                return ErrorOutput {
                    kind: "auth",
                    code: None,
                    message: message.clone(),
                    request_id: None,
                };
            }
            TossError::Api {
                code,
                message,
                request_id,
                ..
            } => {
                return ErrorOutput {
                    kind: "api",
                    code: code.clone(),
                    message: message.clone(),
                    request_id: request_id.clone(),
                };
            }
            TossError::RateLimit {
                message,
                request_id,
                ..
            } => {
                return ErrorOutput {
                    kind: "rate_limit",
                    code: Some("rate-limit-exceeded".to_string()),
                    message: message.clone(),
                    request_id: request_id.clone(),
                };
            }
            TossError::Runtime(message) => {
                return ErrorOutput {
                    kind: "runtime",
                    code: None,
                    message: message.clone(),
                    request_id: None,
                };
            }
            TossError::Io(_) | TossError::Yaml(_) => {
                return ErrorOutput {
                    kind: "config",
                    code: None,
                    message: err.to_string(),
                    request_id: None,
                };
            }
            TossError::Json(_) | TossError::Http(_) => {}
        }
    }

    ErrorOutput {
        kind: "runtime",
        code: None,
        message: err.to_string(),
        request_id: None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use serde_json::json;

    use super::{run_client_command, write_json_error};
    use crate::cli::{self, OutputFormat};
    use toss_core::auth::TokenManager;
    use toss_core::client::TossClient;
    use toss_core::config::{self, AppConfig};
    use toss_core::transport::{HttpRequest, HttpResponse, Transport};

    #[derive(Clone)]
    struct QueueTransport {
        requests: Arc<Mutex<Vec<HttpRequest>>>,
        responses: Arc<Mutex<Vec<HttpResponse>>>,
    }

    #[async_trait]
    impl Transport for QueueTransport {
        async fn send(&self, request: HttpRequest) -> toss_core::Result<HttpResponse> {
            self.requests.lock().unwrap().push(request);
            Ok(self.responses.lock().unwrap().remove(0))
        }
    }

    fn error_kind(err: anyhow::Error) -> String {
        let mut buffer = Vec::new();
        write_json_error(&mut buffer, "config", &err).unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        envelope["error"]["kind"].as_str().unwrap().to_string()
    }

    fn stock_get_command() -> cli::Command {
        cli::Command::Stock(cli::StockArgs {
            command: cli::StockCommand::Get(cli::SymbolArg {
                symbol: "AAPL".to_string(),
            }),
        })
    }

    fn chart_candles_command() -> cli::Command {
        cli::Command::Chart(cli::ChartArgs {
            command: cli::ChartCommand::Candles(cli::CandlesArgs {
                symbol: "AAPL".to_string(),
                interval: cli::CandleInterval::Min1,
                count: Some(200),
                before: Some("2026-06-19T18:20:00+09:00".to_string()),
                adjusted: Some(false),
            }),
        })
    }

    fn token_response() -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: serde_json::to_vec(&json!({
                "access_token": "token-1",
                "token_type": "Bearer",
                "expires_in": 86400
            }))
            .unwrap(),
        }
    }

    fn order_client(
        requests: Arc<Mutex<Vec<HttpRequest>>>,
        responses: Arc<Mutex<Vec<HttpResponse>>>,
        name: &str,
        account_seq: Option<u64>,
    ) -> TossClient<QueueTransport> {
        let transport = QueueTransport {
            requests,
            responses,
        };
        let tempdir = tempfile::tempdir().unwrap();
        let token_manager = TokenManager::new_with_cache_path(
            "client".to_string(),
            "secret".to_string(),
            tempdir.path().join(format!("{name}-token.json")),
            transport.clone(),
        );
        TossClient::new_with_parts(
            AppConfig {
                client_id: "client".to_string(),
                client_secret: "secret".to_string(),
                account_seq,
            },
            token_manager,
            transport,
        )
    }

    fn order_buy_command(dry_run: bool, confirm: bool) -> cli::Command {
        cli::Command::Order(cli::OrderArgs {
            command: cli::OrderCommand::Buy(cli::OrderCreateArgs {
                symbol: "AAPL".to_string(),
                qty: Some("1".to_string()),
                amount: None,
                order_type: cli::OrderType::Limit,
                price: Some("180".to_string()),
                client_order_id: Some("client-1".to_string()),
                dry_run,
                confirm,
                confirm_high_value_order: true,
            }),
        })
    }

    fn order_modify_command(dry_run: bool, confirm: bool) -> cli::Command {
        cli::Command::Order(cli::OrderArgs {
            command: cli::OrderCommand::Modify(cli::OrderModifyArgs {
                order_id: "order-123".to_string(),
                qty: Some("2".to_string()),
                order_type: cli::OrderType::Limit,
                price: Some("181.00".to_string()),
                dry_run,
                confirm,
                confirm_high_value_order: true,
            }),
        })
    }

    fn order_cancel_command(dry_run: bool, confirm: bool) -> cli::Command {
        cli::Command::Order(cli::OrderArgs {
            command: cli::OrderCommand::Cancel(cli::OrderCancelArgs {
                order_id: "order-123".to_string(),
                dry_run,
                confirm,
            }),
        })
    }

    fn order_buying_power_command() -> cli::Command {
        cli::Command::Order(cli::OrderArgs {
            command: cli::OrderCommand::BuyingPower(cli::OrderBuyingPowerArgs {
                currency: "USD".to_string(),
            }),
        })
    }

    fn order_sellable_quantity_command() -> cli::Command {
        cli::Command::Order(cli::OrderArgs {
            command: cli::OrderCommand::SellableQuantity(cli::OrderSellableQuantityArgs {
                symbol: "AAPL".to_string(),
            }),
        })
    }

    fn order_commissions_command() -> cli::Command {
        cli::Command::Order(cli::OrderArgs {
            command: cli::OrderCommand::Commissions,
        })
    }
    fn order_create_command(
        side: cli::OrderSide,
        qty: Option<&str>,
        amount: Option<&str>,
        dry_run: bool,
        confirm: bool,
    ) -> cli::Command {
        let args = cli::OrderCreateArgs {
            symbol: "AAPL".to_string(),
            qty: qty.map(str::to_string),
            amount: amount.map(str::to_string),
            order_type: cli::OrderType::Limit,
            price: Some("180".to_string()),
            client_order_id: Some("client-1".to_string()),
            dry_run,
            confirm,
            confirm_high_value_order: true,
        };
        cli::Command::Order(cli::OrderArgs {
            command: match side {
                cli::OrderSide::Buy => cli::OrderCommand::Buy(args),
                cli::OrderSide::Sell => cli::OrderCommand::Sell(args),
            },
        })
    }

    fn order_list_command() -> cli::Command {
        cli::Command::Order(cli::OrderArgs {
            command: cli::OrderCommand::List(cli::OrderListArgs {
                status: cli::OrderHistoryStatus::Open,
            }),
        })
    }

    fn order_show_command(order_id: &str) -> cli::Command {
        cli::Command::Order(cli::OrderArgs {
            command: cli::OrderCommand::Show(cli::OrderShowArgs {
                order_id: order_id.to_string(),
            }),
        })
    }

    fn order_history_order_json(order_id: &str) -> serde_json::Value {
        json!({
            "orderId": order_id,
            "symbol": "AAPL",
            "side": "BUY",
            "orderType": "LIMIT",
            "timeInForce": "DAY",
            "status": "OPEN",
            "price": "180",
            "quantity": "1",
            "orderAmount": null,
            "currency": "USD",
            "orderedAt": "2026-03-29T09:30:00+09:00",
            "canceledAt": null,
            "execution": {
                "filledQuantity": "0",
                "averageFilledPrice": null,
                "filledAmount": null,
                "commission": null,
                "tax": null,
                "filledAt": null,
                "settlementDate": null
            }
        })
    }

    fn order_history_list_json(order_id: &str) -> serde_json::Value {
        json!({
            "orders": [order_history_order_json(order_id)],
            "nextCursor": null,
            "hasNext": false
        })
    }

    #[tokio::test]
    async fn chart_candles_dispatches_openapi_pagination_query() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: serde_json::to_vec(&json!({
                    "result": { "candles": [] }
                }))
                .unwrap(),
            },
        ]));
        let client = order_client(requests.clone(), responses, "chart-candles", None);

        let mut buffer = Vec::new();
        run_client_command(
            OutputFormat::Json,
            "chart",
            chart_candles_command(),
            &client,
            &mut buffer,
        )
        .await
        .unwrap();

        let envelope: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        assert_eq!(envelope["ok"], true);

        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[1].method, toss_core::transport::HttpMethod::Get);
        assert_eq!(captured[1].path, "/api/v1/candles");
        assert_eq!(
            captured[1].query,
            vec![
                ("symbol".to_string(), "AAPL".to_string()),
                ("interval".to_string(), "1m".to_string()),
                ("count".to_string(), "200".to_string()),
                (
                    "before".to_string(),
                    "2026-06-19T18:20:00+09:00".to_string()
                ),
                ("adjusted".to_string(), "false".to_string())
            ]
        );
    }

    #[tokio::test]
    async fn order_list_dispatches_through_wrapper() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: serde_json::to_vec(&json!({
                    "result": order_history_list_json("order-123")
                }))
                .unwrap(),
            },
        ]));
        let client = order_client(requests.clone(), responses, "order-list", Some(42));

        let mut buffer = Vec::new();
        run_client_command(
            OutputFormat::Json,
            "order",
            order_list_command(),
            &client,
            &mut buffer,
        )
        .await
        .unwrap();

        let envelope: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        assert_eq!(envelope["ok"], true);
        assert_eq!(envelope["command"], "order");
        assert_eq!(envelope["data"]["orders"][0]["orderId"], "order-123");

        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[1].method, toss_core::transport::HttpMethod::Get);
        assert_eq!(captured[1].path, "/api/v1/orders");
        assert_eq!(
            captured[1].query,
            vec![("status".to_string(), "OPEN".to_string())]
        );
        assert_eq!(
            captured[1]
                .headers
                .iter()
                .find(|header| header.name == "X-Tossinvest-Account")
                .map(|header| header.value.as_str()),
            Some("42")
        );
    }

    #[tokio::test]
    async fn order_show_dispatches_through_wrapper() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: serde_json::to_vec(&json!({
                    "result": order_history_order_json("order-123")
                }))
                .unwrap(),
            },
        ]));
        let client = order_client(requests.clone(), responses, "order-show", Some(42));

        let mut buffer = Vec::new();
        run_client_command(
            OutputFormat::Json,
            "order",
            order_show_command("order-123"),
            &client,
            &mut buffer,
        )
        .await
        .unwrap();

        let envelope: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        assert_eq!(envelope["ok"], true);
        assert_eq!(envelope["command"], "order");
        assert_eq!(envelope["data"]["orderId"], "order-123");

        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[1].method, toss_core::transport::HttpMethod::Get);
        assert_eq!(captured[1].path, "/api/v1/orders/order-123");
        assert!(captured[1].query.is_empty());
        assert_eq!(
            captured[1]
                .headers
                .iter()
                .find(|header| header.name == "X-Tossinvest-Account")
                .map(|header| header.value.as_str()),
            Some("42")
        );
    }

    #[tokio::test]
    async fn order_create_rejects_both_qty_and_amount() {
        for side in [cli::OrderSide::Buy, cli::OrderSide::Sell] {
            let requests = Arc::new(Mutex::new(Vec::new()));
            let responses = Arc::new(Mutex::new(Vec::new()));
            let client = order_client(requests.clone(), responses, "order-size-both", Some(42));

            let err = run_client_command(
                OutputFormat::Json,
                "order",
                order_create_command(side, Some("1"), Some("100"), true, false),
                &client,
                &mut Vec::new(),
            )
            .await
            .unwrap_err();

            assert!(err.to_string().contains("exactly one"), "{side:?}: {err}");
            assert!(err.to_string().contains("--qty"), "{side:?}: {err}");
            assert!(err.to_string().contains("--amount"), "{side:?}: {err}");
            assert!(requests.lock().unwrap().is_empty(), "{side:?}: {err}");
        }
    }

    #[tokio::test]
    async fn order_create_rejects_missing_qty_and_amount() {
        for side in [cli::OrderSide::Buy, cli::OrderSide::Sell] {
            let requests = Arc::new(Mutex::new(Vec::new()));
            let responses = Arc::new(Mutex::new(Vec::new()));
            let client = order_client(requests.clone(), responses, "order-size-missing", Some(42));

            let err = run_client_command(
                OutputFormat::Json,
                "order",
                order_create_command(side, None, None, true, false),
                &client,
                &mut Vec::new(),
            )
            .await
            .unwrap_err();

            assert!(err.to_string().contains("exactly one"), "{side:?}: {err}");
            assert!(err.to_string().contains("--qty"), "{side:?}: {err}");
            assert!(err.to_string().contains("--amount"), "{side:?}: {err}");
            assert!(requests.lock().unwrap().is_empty(), "{side:?}: {err}");
        }
    }

    #[test]
    fn build_order_create_request_omits_confirm_high_value_order_when_flag_is_off() {
        let request = super::build_order_create_request(
            &cli::OrderCreateArgs {
                symbol: "AAPL".to_string(),
                qty: Some("1".to_string()),
                amount: None,
                order_type: cli::OrderType::Limit,
                price: Some("180".to_string()),
                client_order_id: Some("client-1".to_string()),
                dry_run: true,
                confirm: false,
                confirm_high_value_order: false,
            },
            super::CoreOrderSide::BUY,
        );

        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["clientOrderId"], "client-1");
        assert!(value.get("confirmHighValueOrder").is_none(), "{value}");
    }

    #[tokio::test]
    async fn dry_run_order_commands_write_safe_json() {
        let cases = [
            (
                "buy",
                order_buy_command(true, true),
                json!({
                    "clientOrderId": "client-1",
                    "symbol": "AAPL",
                    "side": "BUY",
                    "orderType": "LIMIT",
                    "quantity": "1",
                    "price": "180",
                    "confirmHighValueOrder": true
                }),
                "/api/v1/orders",
            ),
            (
                "modify",
                order_modify_command(true, true),
                json!({
                    "orderType": "LIMIT",
                    "quantity": "2",
                    "price": "181.00",
                    "confirmHighValueOrder": true
                }),
                "/api/v1/orders/order-123/modify",
            ),
            (
                "cancel",
                order_cancel_command(true, true),
                json!({}),
                "/api/v1/orders/order-123/cancel",
            ),
        ];

        for (name, command, expected_body, expected_path) in cases {
            let requests = Arc::new(Mutex::new(Vec::new()));
            let responses = Arc::new(Mutex::new(Vec::new()));
            let client = order_client(requests.clone(), responses, name, Some(42));

            let mut buffer = Vec::new();
            run_client_command(OutputFormat::Json, "order", command, &client, &mut buffer)
                .await
                .unwrap();

            let envelope: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
            assert_eq!(envelope["ok"], true);
            assert_eq!(envelope["command"], "order");
            let data = &envelope["data"];
            assert_eq!(data["dryRun"], true);
            assert_eq!(data["method"], "POST");
            assert_eq!(data["path"], expected_path);
            assert_eq!(data["accountHeaderPresent"], true);
            assert_eq!(data["body"], expected_body);
            assert!(data.get("authorization").is_none(), "{envelope}");
            assert!(data.get("token").is_none(), "{envelope}");
            assert!(data.get("clientSecret").is_none(), "{envelope}");
            assert!(data.get("client_id").is_none(), "{envelope}");
            assert!(requests.lock().unwrap().is_empty(), "{envelope}");
        }
    }
    #[tokio::test]
    async fn order_buy_with_confirm_dispatches_live_wrapper() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: serde_json::to_vec(&json!({
                    "result": {
                        "orderId": "order-789"
                    }
                }))
                .unwrap(),
            },
        ]));
        let client = order_client(requests.clone(), responses, "buy-live", Some(42));

        let mut buffer = Vec::new();
        run_client_command(
            OutputFormat::Json,
            "order",
            order_buy_command(false, true),
            &client,
            &mut buffer,
        )
        .await
        .unwrap();

        let envelope: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        assert_eq!(envelope["ok"], true);
        assert_eq!(envelope["command"], "order");
        assert_eq!(envelope["data"], json!({ "orderId": "order-789" }));

        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[1].method, toss_core::transport::HttpMethod::Post);
        assert_eq!(captured[1].path, "/api/v1/orders");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(captured[1].body.as_ref().unwrap())
                .unwrap(),
            json!({
                "clientOrderId": "client-1",
                "symbol": "AAPL",
                "side": "BUY",
                "orderType": "LIMIT",
                "quantity": "1",
                "price": "180",
                "confirmHighValueOrder": true
            })
        );
    }

    #[tokio::test]
    async fn order_buying_power_dispatches_through_wrapper() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: serde_json::to_vec(&json!({
                    "result": {
                        "currency": "USD",
                        "cashBuyingPower": "1234.56"
                    }
                }))
                .unwrap(),
            },
        ]));
        let client = order_client(requests.clone(), responses, "buying-power", Some(42));

        let mut buffer = Vec::new();
        run_client_command(
            OutputFormat::Json,
            "order",
            order_buying_power_command(),
            &client,
            &mut buffer,
        )
        .await
        .unwrap();

        let envelope: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        assert_eq!(envelope["ok"], true);
        assert_eq!(
            envelope["data"],
            json!({
                "currency": "USD",
                "cashBuyingPower": "1234.56"
            })
        );

        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[1].method, toss_core::transport::HttpMethod::Get);
        assert_eq!(captured[1].path, "/api/v1/buying-power");
        assert_eq!(
            captured[1].query,
            vec![("currency".to_string(), "USD".to_string())]
        );
    }

    #[tokio::test]
    async fn order_sellable_quantity_dispatches_through_wrapper() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: serde_json::to_vec(&json!({
                    "result": {
                        "sellableQuantity": "7"
                    }
                }))
                .unwrap(),
            },
        ]));
        let client = order_client(requests.clone(), responses, "sellable-quantity", Some(42));

        let mut buffer = Vec::new();
        run_client_command(
            OutputFormat::Json,
            "order",
            order_sellable_quantity_command(),
            &client,
            &mut buffer,
        )
        .await
        .unwrap();

        let envelope: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        assert_eq!(envelope["ok"], true);
        assert_eq!(
            envelope["data"],
            json!({
                "sellableQuantity": "7"
            })
        );

        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[1].method, toss_core::transport::HttpMethod::Get);
        assert_eq!(captured[1].path, "/api/v1/sellable-quantity");
        assert_eq!(
            captured[1].query,
            vec![("symbol".to_string(), "AAPL".to_string())]
        );
    }

    #[tokio::test]
    async fn order_commissions_dispatches_through_wrapper() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: serde_json::to_vec(&json!({
                    "result": [
                        {
                            "marketCountry": "US",
                            "commissionRate": "0.001"
                        }
                    ]
                }))
                .unwrap(),
            },
        ]));
        let client = order_client(requests.clone(), responses, "commissions", Some(42));

        let mut buffer = Vec::new();
        run_client_command(
            OutputFormat::Json,
            "order",
            order_commissions_command(),
            &client,
            &mut buffer,
        )
        .await
        .unwrap();

        let envelope: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        assert_eq!(envelope["ok"], true);
        assert_eq!(
            envelope["data"],
            json!([
                {
                    "marketCountry": "US",
                    "commissionRate": "0.001"
                }
            ])
        );

        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[1].method, toss_core::transport::HttpMethod::Get);
        assert_eq!(captured[1].path, "/api/v1/commissions");
        assert!(captured[1].query.is_empty());
    }

    #[tokio::test]
    async fn order_buy_without_dry_run_or_confirm_is_rejected() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(Vec::new()));
        let client = order_client(requests, responses, "missing-safety", Some(42));

        let err = run_client_command(
            OutputFormat::Json,
            "order",
            order_buy_command(false, false),
            &client,
            &mut Vec::new(),
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("dry-run"), "{err}");
        assert!(err.to_string().contains("confirm"), "{err}");
    }

    #[test]
    fn classifies_io_config_failures_as_config() {
        let err = anyhow::Error::new(toss_core::TossError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "missing config",
        )));

        assert_eq!(error_kind(err), "config");
    }

    #[test]
    fn classifies_yaml_config_failures_as_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::write(&path, "client_id: [1, 2").unwrap();
        let err = config::load(Some(&path), None).unwrap_err();

        assert_eq!(error_kind(anyhow::Error::new(err)), "config");
    }

    #[tokio::test]
    async fn json_output_preserves_absent_optional_fields() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: serde_json::to_vec(&json!({
                    "access_token": "token-1",
                    "token_type": "Bearer",
                    "expires_in": 86400
                }))
                .unwrap(),
            },
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: serde_json::to_vec(&json!({
                    "result": [
                        {
                            "symbol": "AAPL",
                            "name": "Apple",
                            "englishName": "APPLE INC",
                            "isinCode": "US0378331005",
                            "market": "NASDAQ",
                            "securityType": "COMMON",
                            "isCommonShare": true,
                            "status": "ACTIVE",
                            "currency": "USD",
                            "sharesOutstanding": "100",
                            "koreanMarketDetail": {
                                "liquidationTrading": false,
                                "nxtSupported": true,
                                "krxTradingSuspended": false
                            }
                        }
                    ]
                }))
                .unwrap(),
            },
        ]));
        let transport = QueueTransport {
            requests: requests.clone(),
            responses,
        };
        let tempdir = tempfile::tempdir().unwrap();
        let token_manager = TokenManager::new_with_cache_path(
            "client".to_string(),
            "secret".to_string(),
            tempdir.path().join("token.json"),
            transport.clone(),
        );
        let client = TossClient::new_with_parts(
            AppConfig {
                client_id: "client".to_string(),
                client_secret: "secret".to_string(),
                account_seq: None,
            },
            token_manager,
            transport,
        );

        let mut buffer = Vec::new();
        run_client_command(
            OutputFormat::Json,
            "stock",
            stock_get_command(),
            &client,
            &mut buffer,
        )
        .await
        .unwrap();

        let envelope: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        let detail = envelope["data"][0]["koreanMarketDetail"]
            .as_object()
            .unwrap();
        assert_eq!(detail["liquidationTrading"], false);
        assert!(detail.get("nxtTradingSuspended").is_none(), "{envelope}");

        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[1].path, "/api/v1/stocks");
        assert_eq!(
            captured[1].query,
            vec![("symbols".to_string(), "AAPL".to_string())]
        );
    }
    #[tokio::test]
    async fn rejects_incomplete_typed_stock_payloads() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: serde_json::to_vec(&json!({
                    "access_token": "token-1",
                    "token_type": "Bearer",
                    "expires_in": 86400
                }))
                .unwrap(),
            },
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: serde_json::to_vec(&json!({
                    "result": [
                        {
                            "symbol": "AAPL",
                            "englishName": "APPLE INC",
                            "isinCode": "US0378331005",
                            "market": "NASDAQ",
                            "securityType": "COMMON",
                            "isCommonShare": true,
                            "status": "ACTIVE",
                            "currency": "USD",
                            "sharesOutstanding": "100"
                        }
                    ]
                }))
                .unwrap(),
            },
        ]));
        let transport = QueueTransport {
            requests: requests.clone(),
            responses,
        };
        let tempdir = tempfile::tempdir().unwrap();
        let token_manager = TokenManager::new_with_cache_path(
            "client".to_string(),
            "secret".to_string(),
            tempdir.path().join("token.json"),
            transport.clone(),
        );
        let client = TossClient::new_with_parts(
            AppConfig {
                client_id: "client".to_string(),
                client_secret: "secret".to_string(),
                account_seq: None,
            },
            token_manager,
            transport,
        );

        let err = run_client_command(
            OutputFormat::Json,
            "stock",
            stock_get_command(),
            &client,
            &mut Vec::new(),
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("missing field"), "{err}");
    }
}

fn mask_client_id(client_id: &str) -> String {
    if client_id.len() <= 8 {
        return "****".to_string();
    }

    format!(
        "{}****{}",
        &client_id[..4],
        &client_id[client_id.len() - 4..]
    )
}
