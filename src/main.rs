mod backend;
mod definition;
mod diagnostics;
mod document;
mod hover;
mod rename;
mod semantic_tokens;
mod symbols;

use clap::Parser;
use tower_lsp::{LspService, Server};

#[derive(Parser)]
#[command(name = "rholang-lsp", about = "Rholang Language Server")]
struct Cli {
    /// Use stdio transport
    #[arg(long)]
    stdio: bool,

    /// Log level
    #[arg(long, default_value = "warn")]
    log_level: String,

    /// Disable color output
    #[arg(long)]
    no_color: bool,

    /// Client process ID (exit when client dies)
    #[arg(long)]
    client_process_id: Option<u32>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(match cli.log_level.as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => log::LevelFilter::Warn,
        })
        .init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(backend::Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
