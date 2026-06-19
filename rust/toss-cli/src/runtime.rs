use std::io::Write;

use anyhow::Result;
use serde::Serialize;
use serde_json::json;
use toss_core::TossError;
use toss_core::client::TossClient;
use toss_core::config::{self, AppConfig};
use toss_core::{account, asset, market_data, market_info, stock_info};

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
    code: Option<String>,
    message: String,
    #[serde(rename = "requestId")]
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
    let app_config = config::load(config.as_deref(), account.as_deref())?;

    match command {
        cli::Command::Config => run_config(output_format, command_name, &app_config, writer),
        cli::Command::Auth(args) => match args.command {
            cli::AuthCommand::Token => {
                let client = TossClient::new(app_config)?;
                client.check_token().await?;
                write_output(
                    output_format,
                    command_name,
                    json!({ "token_check": "ok" }),
                    writer,
                )
            }
        },
        cli::Command::Price(args) => {
            let client = TossClient::new(app_config)?;
            let symbols = args.symbols.as_deref().unwrap_or(&args.symbol);
            let value = market_data::prices(&client, symbols).await?;
            write_output(output_format, command_name, value, writer)
        }
        cli::Command::Quote(args) => {
            let client = TossClient::new(app_config)?;
            let value = match args.command {
                cli::QuoteCommand::Orderbook(arg) => {
                    market_data::orderbook_json(&client, &arg.symbol).await?
                }
                cli::QuoteCommand::Trades(arg) => {
                    market_data::trades_json(&client, &arg.symbol).await?
                }
                cli::QuoteCommand::Limits(arg) => {
                    market_data::price_limits_json(&client, &arg.symbol).await?
                }
            };
            write_output(output_format, command_name, value, writer)
        }
        cli::Command::Chart(args) => {
            let client = TossClient::new(app_config)?;
            let value = match args.command {
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
                    market_data::candles(&client, query).await?
                }
            };
            write_output(output_format, command_name, value, writer)
        }
        cli::Command::Stock(args) => {
            let client = TossClient::new(app_config)?;
            let value = match args.command {
                cli::StockCommand::Get(arg) => stock_info::stocks(&client, &arg.symbol).await?,
                cli::StockCommand::Warnings(arg) => {
                    stock_info::warnings(&client, &arg.symbol).await?
                }
                cli::StockCommand::Search(arg) => stock_info::stocks(&client, &arg.symbols).await?,
            };
            write_output(output_format, command_name, value, writer)
        }
        cli::Command::Market(args) => {
            let client = TossClient::new(app_config)?;
            let value = match args.command {
                cli::MarketCommand::ExchangeRate => market_info::exchange_rate(&client).await?,
                cli::MarketCommand::Calendar(args) => match args.command {
                    cli::CalendarCommand::Kr => market_info::kr_calendar(&client).await?,
                    cli::CalendarCommand::Us => market_info::us_calendar(&client).await?,
                },
            };
            write_output(output_format, command_name, value, writer)
        }
        cli::Command::Account(args) => match args.command {
            cli::AccountCommand::Use(args) => {
                let path = config::save_account_seq(config.as_deref(), args.account_seq)?;
                write_output(
                    output_format,
                    command_name,
                    json!({ "config_path": path, "account_seq": args.account_seq }),
                    writer,
                )
            }
            cli::AccountCommand::List => {
                let client = TossClient::new(app_config)?;
                let value = account::list(&client).await?;
                write_output(output_format, command_name, value, writer)
            }
        },
        cli::Command::Holdings => {
            let client = TossClient::new(app_config)?;
            let value = asset::holdings(&client).await?;
            write_output(output_format, command_name, value, writer)
        }
    }
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

fn write_output<T: Serialize>(
    output_format: OutputFormat,
    command: &str,
    data: T,
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
            let value = serde_json::to_value(data)?;
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

    use super::write_json_error;
    use toss_core::TossError;
    use toss_core::config;

    fn error_kind(err: anyhow::Error) -> String {
        let mut buffer = Vec::new();
        write_json_error(&mut buffer, "config", &err).unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        envelope["error"]["kind"].as_str().unwrap().to_string()
    }

    #[test]
    fn classifies_io_config_failures_as_config() {
        let err = anyhow::Error::new(TossError::Io(io::Error::new(
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
