//! # VARCAVIA Node
//!
//! Binary principale che avvia un nodo VARCAVIA.
//! Integra tutti i componenti: dDNA, VTP, ARC, CDE, UAG.

use clap::Parser;
use tracing_subscriber::EnvFilter;

mod config;
mod storage;
mod network;
mod cli;

/// Nodo VARCAVIA — Sistema Planetario di Dati Puliti
#[derive(Parser, Debug)]
#[command(name = "varcavia-node", version, about)]
struct Args {
    /// Path al file di configurazione
    #[arg(short, long, default_value = "configs/node_default.toml")]
    config: String,

    /// Porta di ascolto (override config)
    #[arg(short, long)]
    port: Option<u16>,

    /// Livello di log (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Option<cli::Commands>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&args.log_level)),
        )
        .init();

    tracing::info!("VARCAVIA Node v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Config: {}", args.config);

    match args.command {
        Some(cli::Commands::Init { data_dir }) => {
            cli::handle_init(&data_dir).await?;
        }
        Some(cli::Commands::Status) => {
            cli::handle_status().await?;
        }
        None => {
            tracing::info!("Avvio nodo VARCAVIA...");
            // TODO: avvio completo del nodo
            tracing::info!("Nodo avviato. Premi Ctrl+C per terminare.");
            tokio::signal::ctrl_c().await?;
            tracing::info!("Shutdown in corso...");
        }
    }

    Ok(())
}
