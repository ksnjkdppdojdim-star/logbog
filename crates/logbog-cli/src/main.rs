mod commands;
mod output;

use clap::Parser;
use commands::Cli;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    // Initialiser le tracing via LOGBOG_LOG ou RUST_LOG
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("LOGBOG_LOG").unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_target(false)
        .compact()
        .init();

    let cli = Cli::parse();
    commands::run(cli)
}
