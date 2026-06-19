use std::io::Write;

use anyhow::Result;
use serde::Serialize;
use serde_json::json;
use toss_core::config::{self, AppConfig};
use toss_core::TossError;

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
            cli::AccountCommand::List => Err(anyhow::anyhow!("network commands are implemented in Task 5")),
        },
        _ => Err(anyhow::anyhow!("network commands are implemented in Task 5")),
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
            serde_json::to_writer(&mut *writer, &SuccessEnvelope { ok: true, command, data })?;
            writeln!(&mut *writer)?;
        }
        OutputFormat::Text => {
            let value = serde_json::to_value(data)?;
            if command == "config" {
                render::write_key_values(
                    writer,
                    &[
                        ("client_id", value["client_id"].as_str().unwrap_or("-").to_string()),
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
    serde_json::to_writer(&mut *writer, &ErrorEnvelope { ok: false, command, error })?;
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
            TossError::Io(_) | TossError::Yaml(_) | TossError::Json(_) | TossError::Http(_) => {}
        }
    }

    ErrorOutput {
        kind: "runtime",
        code: None,
        message: err.to_string(),
        request_id: None,
    }
}

fn mask_client_id(client_id: &str) -> String {
    if client_id.len() <= 8 {
        return "****".to_string();
    }

    format!("{}****{}", &client_id[..4], &client_id[client_id.len() - 4..])
}
