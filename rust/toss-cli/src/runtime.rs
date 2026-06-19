use std::io::Write;

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
use toss_core::stock_info;
use toss_core::transport::Transport;

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
    if matches!(command, cli::Command::Order(_)) {
        return Err(anyhow::anyhow!(
            "order command dispatch is not implemented yet"
        ));
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
                if let Some(from) = args.from {
                    query.push(("from".to_string(), from));
                }
                if let Some(to) = args.to {
                    query.push(("to".to_string(), to));
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
        cli::Command::Order(_) => {
            return Err(anyhow::anyhow!(
                "order command dispatch is not implemented yet"
            ));
        }
        cli::Command::Holdings => serde_json::to_value(asset::holdings(client).await?)?,
        cli::Command::Config => unreachable!("config is handled before client dispatch"),
    };
    write_output(output_format, command_name, value, writer)
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
