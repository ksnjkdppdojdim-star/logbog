pub mod init;
pub mod install;
pub mod list;
pub mod pack;
pub mod query;
pub mod remove;
pub mod start;
pub mod status;
pub mod tail;

use clap::{Parser, Subcommand};

/// LogBog — Modular server log management framework.
#[derive(Parser)]
#[command(
    name = "logbog",
    version,
    about = "Modular server log management with intelligence",
    long_about = "LogBog collecte, parse, corrèle et analyse automatiquement les logs de votre serveur.\n\nInstallation rapide :\n  logbog init\n  logbog install nginx php-fpm mysql\n  logbog start"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Configuration file path
    #[arg(short, long, global = true, default_value = "logbog.toml")]
    pub config: String,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize LogBog in the current directory
    Init {
        /// Data directory path
        #[arg(long, default_value = "/var/lib/logbog")]
        data_dir: String,

        /// Server port
        #[arg(long, default_value_t = 6060)]
        port: u16,
    },

    /// Start the LogBog collection pipeline
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
    },

    /// Stop the LogBog service
    Stop,

    /// Show service status and storage statistics
    Status,

    /// Install one or more log packs
    Install {
        /// Pack names to install
        packs: Vec<String>,
    },

    /// Remove a log pack
    Remove {
        /// Pack name to remove
        pack: String,
    },

    /// List installed packs
    List,

    /// Pack management commands
    Pack {
        #[command(subcommand)]
        action: pack::PackAction,
    },

    /// Show or edit configuration
    Config {
        /// Show the current configuration
        #[arg(long)]
        show: bool,
    },

    /// Query stored logs
    Query {
        /// Raw SQL query (e.g., "SELECT * FROM logs WHERE level = 'error' LIMIT 10")
        #[arg(long)]
        sql: Option<String>,

        /// Filter by source name
        #[arg(short, long)]
        source: Option<String>,

        /// Filter by pack name
        #[arg(short, long)]
        pack: Option<String>,

        /// Minimum log level (trace, debug, info, warn, error, fatal)
        #[arg(short, long)]
        level: Option<String>,

        /// Search in message text
        #[arg(long)]
        search: Option<String>,

        /// Maximum number of results
        #[arg(short = 'n', long, default_value_t = 50)]
        limit: usize,
    },

    /// Show the most recent logs (like tail -f)
    Tail {
        /// Filter by source name
        #[arg(short, long)]
        source: Option<String>,

        /// Filter by pack name
        #[arg(short, long)]
        pack: Option<String>,

        /// Number of lines to show
        #[arg(short = 'n', long, default_value_t = 20)]
        lines: usize,
    },
}

pub fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Init { data_dir, port } => init::run(&data_dir, port),
        Command::Start { foreground } => start::run(&cli.config, foreground),
        Command::Stop => {
            crate::output::info("Stopping LogBog service...");
            crate::output::warn("Service management not yet implemented (Phase 2)");
            Ok(())
        }
        Command::Status => status::run(&cli.config),
        Command::Install { packs } => install::run(&cli.config, &packs),
        Command::Remove { pack } => remove::run(&cli.config, &pack),
        Command::List => list::run(&cli.config),
        Command::Pack { action } => pack::run(action),
        Command::Config { show } => {
            if show {
                let config = logbog_core::Config::load(std::path::Path::new(&cli.config))?;
                let toml_str = toml::to_string_pretty(&config)?;
                println!("{toml_str}");
            } else {
                crate::output::info(&format!("Configuration file: {}", cli.config));
                crate::output::info("Use --show to display the current configuration");
            }
            Ok(())
        }
        Command::Query {
            sql,
            source,
            pack,
            level,
            search,
            limit,
        } => query::run(
            &cli.config,
            sql.as_deref(),
            source.as_deref(),
            pack.as_deref(),
            level.as_deref(),
            search.as_deref(),
            limit,
        ),
        Command::Tail {
            source,
            pack,
            lines,
        } => tail::run(&cli.config, source.as_deref(), pack.as_deref(), lines),
    }
}
