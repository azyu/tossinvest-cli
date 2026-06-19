use std::ffi::{OsStr, OsString};

use clap::{Parser, error::ErrorKind};
use toss_cli::cli::{Cli, OutputFormat};
use toss_core::TossError;

#[tokio::main]
async fn main() {
    let args: Vec<OsString> = std::env::args_os().collect();
    let mut stdout = std::io::stdout();

    match Cli::try_parse_from(args.clone()) {
        Ok(cli) => {
            let command = cli.command.name();
            let output = cli.output_format();

            if let Err(error) = toss_cli::runtime::run(cli, &mut stdout).await {
                match output {
                    OutputFormat::Json => {
                        let _ = toss_cli::runtime::write_json_error(&mut stdout, command, &error);
                    }
                    OutputFormat::Text => eprintln!("{error}"),
                }
                std::process::exit(1);
            }
        }
        Err(error) => {
            if emits_json_validation_error(&args, error.kind()) {
                let command = command_name(&args);
                let error = anyhow::Error::new(TossError::Validation(error.to_string()));
                let _ = toss_cli::runtime::write_json_error(&mut stdout, &command, &error);
                std::process::exit(1);
            }
            error.exit();
        }
    }
}

fn emits_json_validation_error(args: &[OsString], kind: ErrorKind) -> bool {
    json_requested(args) && !matches!(kind, ErrorKind::DisplayHelp | ErrorKind::DisplayVersion)
}

fn json_requested(args: &[OsString]) -> bool {
    let mut skip_value = false;
    for arg in args.iter().skip(1) {
        let value = arg.to_string_lossy();
        if skip_value {
            skip_value = false;
            if value == "json" {
                return true;
            }
            continue;
        }
        match value.as_ref() {
            "--json" | "--output=json" => return true,
            "--output" => skip_value = true,
            _ => {}
        }
    }
    false
}

fn command_name(args: &[OsString]) -> String {
    let mut skip_value = false;
    for arg in args.iter().skip(1) {
        let value = arg.to_string_lossy();
        if skip_value {
            skip_value = false;
            continue;
        }
        if value == "--config" || value == "--account" || value == "--output" {
            skip_value = true;
            continue;
        }
        if value.starts_with("--config=")
            || value.starts_with("--account=")
            || value.starts_with("--output=")
        {
            continue;
        }
        match value.as_ref() {
            "--json" | "--quiet" | "--help" | "-h" | "--version" | "-V" => {}
            _ if value.starts_with('-') => {}
            _ => return value.into_owned(),
        }
    }
    "unknown".to_string()
}
