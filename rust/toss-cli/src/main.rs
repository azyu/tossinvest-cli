use clap::Parser;
use toss_cli::cli::{Cli, OutputFormat};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let command = cli.command.name();
    let output = cli.output_format();
    let mut stdout = std::io::stdout();

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
