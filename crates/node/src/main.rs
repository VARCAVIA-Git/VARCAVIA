//! # VARCAVIA Node
//!
//! Binary principale che avvia un nodo VARCAVIA.
//! Integra tutti i componenti: dDNA, VTP, ARC, CDE, UAG.

use clap::Parser;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;
use varcavia_cde::pipeline::PipelineConfig;
use varcavia_ddna::identity::KeyPair;
use varcavia_uag::state::AppState;

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

    /// Directory per i dati del nodo
    #[arg(short, long, default_value = "~/varcavia-data")]
    data_dir: String,

    /// Porta di ascolto API (default: 8080)
    #[arg(short, long)]
    port: Option<u16>,

    /// Livello di log (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Lista di peer P2P da contattare all'avvio (es. 127.0.0.1:8181,127.0.0.1:8182)
    #[arg(long, value_delimiter = ',')]
    peers: Vec<String>,

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

    match args.command {
        Some(cli::Commands::Init { data_dir }) => {
            cli::handle_init(&data_dir).await?;
        }
        Some(cli::Commands::Status) => {
            cli::handle_status().await?;
        }
        None => {
            run_node(args).await?;
        }
    }

    Ok(())
}

/// Avvia il nodo completo: storage + network + API server.
async fn run_node(args: Args) -> anyhow::Result<()> {
    let data_dir = shellexpand::tilde(&args.data_dir).to_string();
    std::fs::create_dir_all(&data_dir)?;

    // 1. Apri (o crea) il database sled
    let db_path = format!("{data_dir}/db");
    let db = sled::open(&db_path)?;
    tracing::info!("Storage aperto: {}", db_path);

    // 2. Carica o genera keypair del nodo
    let key_path = format!("{data_dir}/node_key.secret");
    let keypair = if std::path::Path::new(&key_path).exists() {
        let secret_bytes = std::fs::read(&key_path)?;
        let secret: [u8; 32] = secret_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Chiave privata corrotta (attesi 32 bytes)"))?;
        KeyPair::from_bytes(&secret)
    } else {
        let kp = KeyPair::generate();
        std::fs::write(&key_path, kp.secret_bytes())?;
        tracing::info!("Nuova chiave generata e salvata in: {}", key_path);
        kp
    };
    let node_id = hex::encode(keypair.public_key_bytes());
    tracing::info!("Node ID: {}...{}", &node_id[..8], &node_id[56..]);

    // 3. Crea la pipeline CDE
    let pipeline_config = PipelineConfig::default();

    // 4. Crea lo stato condiviso
    let state = Arc::new(AppState::new(db, keypair.secret_bytes(), pipeline_config));

    // 5. Avvia il NetworkManager TCP
    let api_port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .or(args.port)
        .unwrap_or(8080);
    let net_port = api_port + 100; // P2P su porta API + 100
    let net_addr = format!("127.0.0.1:{net_port}").parse()?;
    let network_mgr = network::NetworkManager::new(
        node_id,
        net_addr,
        state.clone(),
    );
    network_mgr.start_listener().await?;

    // 6. Connetti ai bootstrap peers
    if !args.peers.is_empty() {
        let peer_addrs: Vec<std::net::SocketAddr> = args
            .peers
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();
        tracing::info!("Connessione a {} bootstrap peers...", peer_addrs.len());
        network_mgr.connect_to_peers(&peer_addrs).await;
        let connected = state.get_peers().await.len();
        tracing::info!("Connesso a {} peer", connected);
    }

    // 7. Avvia il server UAG (Axum)
    let server_config = varcavia_uag::server::ServerConfig {
        bind_addr: format!("127.0.0.1:{api_port}").parse()?,
        cors_origins: vec!["http://localhost:5173".into()],
        rate_limit_per_sec: 100,
    };

    tracing::info!("Nodo VARCAVIA avviato.");
    tracing::info!("  API server: http://127.0.0.1:{api_port}");
    tracing::info!("  P2P network: 127.0.0.1:{net_port}");
    tracing::info!("  Data dir: {data_dir}");
    tracing::info!("Premi Ctrl+C per terminare.");

    // Avvia server HTTP (bloccante) con graceful shutdown
    let server_state = state.clone();
    let server_handle = tokio::spawn(async move {
        if let Err(e) = varcavia_uag::server::run(server_config, server_state).await {
            tracing::error!("Errore server UAG: {}", e);
        }
    });

    // Aspetta Ctrl+C, poi graceful shutdown con timeout 5s
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutdown in corso (max 5s)...");

    // Abort del server HTTP
    server_handle.abort();

    // Flush storage con timeout
    let flush_state = state.clone();
    let flush_result = tokio::time::timeout(
        tokio::time::Duration::from_secs(5),
        tokio::task::spawn_blocking(move || flush_state.db.flush()),
    )
    .await;

    match flush_result {
        Ok(Ok(Ok(_))) => tracing::info!("Storage flushed."),
        Ok(Ok(Err(e))) => tracing::warn!("Errore flush storage: {}", e),
        Ok(Err(e)) => tracing::warn!("Errore flush task: {}", e),
        Err(_) => tracing::warn!("Flush timeout (5s) — dati recenti potrebbero essere persi"),
    }

    tracing::info!("Nodo terminato.");
    Ok(())
}
