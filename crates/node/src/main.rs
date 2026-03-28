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
        Some(cli::Commands::Seed { port }) => {
            cli::handle_seed(port).await?;
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
    // VARCAVIA_DB_PATH env var overrides default (for Railway persistent volumes)
    let preferred_path = std::env::var("VARCAVIA_DB_PATH")
        .unwrap_or_else(|_| format!("{data_dir}/db"));
    std::fs::create_dir_all(&preferred_path).ok();
    let (db, db_path) = match sled::open(&preferred_path) {
        Ok(db) => {
            tracing::info!("Storage aperto: {}", preferred_path);
            (db, preferred_path)
        }
        Err(e) => {
            let fallback = format!("{data_dir}/db");
            tracing::warn!(
                "Impossibile aprire {}: {} — fallback a {}",
                preferred_path, e, fallback
            );
            std::fs::create_dir_all(&fallback).ok();
            let db = sled::open(&fallback)?;
            tracing::info!("Storage aperto (fallback): {}", fallback);
            (db, fallback)
        }
    };

    // 2. Carica o genera keypair del nodo
    // Store key alongside the DB for persistence
    let key_path = if std::env::var("VARCAVIA_DB_PATH").is_ok() {
        format!("{db_path}/node_key.secret")
    } else {
        format!("{data_dir}/node_key.secret")
    };
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
    let net_addr = format!("0.0.0.0:{net_port}").parse()?;
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
        bind_addr: format!("0.0.0.0:{api_port}").parse()?,
        cors_origins: vec!["http://localhost:5173".into()],
        rate_limit_per_sec: 100,
    };

    tracing::info!("Nodo VARCAVIA avviato.");
    tracing::info!("  API server: http://0.0.0.0:{api_port}");
    tracing::info!("  P2P network: 0.0.0.0:{net_port}");
    tracing::info!("  Data dir: {data_dir}");
    tracing::info!("Premi Ctrl+C per terminare.");

    // Avvia server HTTP (bloccante) con graceful shutdown
    let server_state = state.clone();
    let server_handle = tokio::spawn(async move {
        if let Err(e) = varcavia_uag::server::run(server_config, server_state).await {
            tracing::error!("Errore server UAG: {}", e);
        }
    });

    // 8. Auto-seed: pulisci dati spazzatura e inserisci seed facts mancanti
    {
        let seed_state = state.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            auto_seed(&seed_state);
        });
    }

    // 9. Start Semantic Spider (runs 24/7)
    {
        let spider_state = state.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            run_semantic_spider(spider_state).await;
        });
    }

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

/// Pulisce dati spazzatura e inserisce seed facts mancanti.
fn auto_seed(state: &AppState) {
    use varcavia_uag::state::{PREFIX_DATA, PREFIX_DDNA, PREFIX_INFO, PREFIX_TRUST};
    use std::collections::HashSet;

    let seed_facts = varcavia_crawler::get_seed_facts();
    let keypair = state.keypair();

    // Build set of valid content strings from seed facts
    let valid_contents: HashSet<String> = seed_facts.iter().map(|(c, _)| c.clone()).collect();

    // Step 1: Rimuovi dati non nei seed facts
    let mut removed = 0u64;
    let mut to_remove: Vec<String> = Vec::new();
    for (key, val) in state.db.scan_prefix(PREFIX_DATA).flatten() {
        let id = String::from_utf8_lossy(&key[PREFIX_DATA.len()..]).to_string();
        let content = String::from_utf8_lossy(&val).to_string();
        if !valid_contents.contains(&content) {
            to_remove.push(id);
        }
    }
    for id in &to_remove {
        let _ = state.db.remove(AppState::make_key(PREFIX_DATA, id));
        let _ = state.db.remove(AppState::make_key(PREFIX_DDNA, id));
        let _ = state.db.remove(AppState::make_key(PREFIX_INFO, id));
        let _ = state.db.remove(AppState::make_key(PREFIX_TRUST, id));
        removed += 1;
    }
    if removed > 0 {
        tracing::info!("Auto-seed: rimossi {} dati non autorizzati", removed);
    }

    // Step 2: Inserisci seed facts mancanti
    let mut inserted = 0u64;
    for (content, domain) in &seed_facts {
        let content_bytes = content.as_bytes();

        let ddna = match varcavia_ddna::DataDna::create(content_bytes, &keypair) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let data_id = ddna.id();

        // Gia presente? Skip
        let data_key = AppState::make_key(PREFIX_DATA, &data_id);
        if state.db.get(&data_key).ok().flatten().is_some() {
            continue;
        }

        // Pipeline CDE
        let score = {
            let mut pipeline = state.pipeline.lock().unwrap();
            match pipeline.process(content_bytes, &ddna, domain) {
                Ok(result) => result.score.overall,
                Err(_) => continue,
            }
        };

        let Ok(ddna_bytes) = ddna.to_bytes() else { continue };

        let _ = state.db.insert(data_key, content_bytes);
        let _ = state.db.insert(AppState::make_key(PREFIX_DDNA, &data_id), ddna_bytes.as_slice());

        let info = varcavia_uag::rest::DataInfo {
            domain: domain.clone(),
            score,
            inserted_at_us: chrono::Utc::now().timestamp_micros(),
            verification_count: 1,
        };
        if let Ok(j) = serde_json::to_vec(&info) {
            let _ = state.db.insert(AppState::make_key(PREFIX_INFO, &data_id), j);
        }

        // Crea TrustRecord T1 con attestazione PeerReviewed dal nodo
        let now = chrono::Utc::now().timestamp_micros();
        let mut trust = varcavia_uag::trust::TrustRecord::new(now);
        trust.attestations.push(varcavia_uag::trust::Attestation {
            source_pubkey: state.node_id.clone(),
            domain: domain.clone(),
            source_tier: varcavia_uag::trust::SourceTier::PeerReviewed,
            timestamp_us: now,
        });
        trust.tier = varcavia_uag::trust::compute_tier(&trust, now);
        let trust_key = AppState::make_key(PREFIX_TRUST, &data_id);
        if let Ok(j) = serde_json::to_vec(&trust) {
            let _ = state.db.insert(trust_key, j);
        }

        state.facts_ingested.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        inserted += 1;
    }

    let total = state.data_count();
    tracing::info!(
        "Auto-seed completato: {} inseriti, {} rimossi, {} totali nel DB (tutti T1)",
        inserted, removed, total
    );
}

/// Semantic Spider — crawls topics 24/7, inserts facts, detects contradictions.
async fn run_semantic_spider(state: Arc<AppState>) {
    use varcavia_uag::state::{PREFIX_DATA, PREFIX_DDNA, PREFIX_INFO, PREFIX_TRUST};
    use varcavia_crawler::spider::{SemanticSpider, crawl_topic};
    use std::sync::atomic::Ordering;

    let spider = SemanticSpider::with_db(&state.db);
    tracing::info!("Semantic Spider started (queue: {})", spider.queue_size().await);

    let mut cycle = 0u64;
    loop {
        let seed = match spider.next_seed().await {
            Some(s) => s,
            None => {
                tracing::info!("Spider: queue empty after {} cycles, reseeding", cycle);
                spider.reseed().await;
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                continue;
            }
        };

        if seed.depth > 8 || spider.is_visited(&seed.topic).await {
            continue;
        }
        spider.mark_visited(&seed.topic).await;

        // Crawl this topic
        let result = crawl_topic(&seed.topic, &seed.domain).await;
        spider.stats.topics_crawled.fetch_add(1, Ordering::Relaxed);
        state.spider_topics.fetch_add(1, Ordering::Relaxed);

        let keypair = state.keypair();
        for fact in &result.facts {
            spider.stats.facts_discovered.fetch_add(1, Ordering::Relaxed);
            let content_bytes = fact.text.as_bytes();
            let ddna = match varcavia_ddna::DataDna::create(content_bytes, &keypair) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let data_id = ddna.id();

            // Check if fact exists
            let data_key = AppState::make_key(PREFIX_DATA, &data_id);
            if state.db.get(&data_key).ok().flatten().is_some() {
                // Existing fact — add attestation if new source
                let trust_key = AppState::make_key(PREFIX_TRUST, &data_id);
                if let Some(bytes) = state.db.get(&trust_key).ok().flatten() {
                    if let Ok(mut trust) = serde_json::from_slice::<varcavia_uag::trust::TrustRecord>(&bytes) {
                        if !trust.attestations.iter().any(|a| a.source_pubkey == fact.source) {
                            let now = chrono::Utc::now().timestamp_micros();
                            trust.attestations.push(varcavia_uag::trust::Attestation {
                                source_pubkey: fact.source.clone(),
                                domain: fact.domain.clone(),
                                source_tier: varcavia_uag::trust::SourceTier::Website,
                                timestamp_us: now,
                            });
                            trust.last_updated_us = now;
                            trust.tier = varcavia_uag::trust::compute_tier(&trust, now);
                            if let Ok(j) = serde_json::to_vec(&trust) {
                                let _ = state.db.insert(&trust_key, j);
                            }
                            spider.stats.facts_attested.fetch_add(1, Ordering::Relaxed);
                            state.spider_attested.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                continue;
            }

            // Check for contradiction with similar existing facts (sample 100)
            for (k, v) in state.db.scan_prefix(PREFIX_DATA).flatten().take(100) {
                let existing = String::from_utf8_lossy(&v);
                if let Some(info) = varcavia_crawler::spider::detect_contradiction(&existing, &fact.text) {
                    let ex_id = String::from_utf8_lossy(&k[PREFIX_DATA.len()..]).to_string();
                    tracing::debug!(
                        "Contradiction: '{}' vs '{}' ({:.1}% divergence)",
                        &ex_id[..16.min(ex_id.len())], &fact.text[..40.min(fact.text.len())], info.divergence_pct
                    );
                    spider.stats.contradictions_found.fetch_add(1, Ordering::Relaxed);
                    state.spider_contradictions.fetch_add(1, Ordering::Relaxed);
                    break;
                }
            }
            // New fact — CDE pipeline
            let score = {
                let mut pipeline = state.pipeline.lock().unwrap();
                match pipeline.process(content_bytes, &ddna, &fact.domain) {
                    Ok(r) => r.score.overall,
                    Err(_) => continue,
                }
            };

            let Ok(ddna_bytes) = ddna.to_bytes() else { continue };
            let _ = state.db.insert(data_key, content_bytes);
            let _ = state.db.insert(AppState::make_key(PREFIX_DDNA, &data_id), ddna_bytes.as_slice());
            let info = varcavia_uag::rest::DataInfo {
                domain: fact.domain.clone(), score,
                inserted_at_us: chrono::Utc::now().timestamp_micros(),
                verification_count: 1,
            };
            if let Ok(j) = serde_json::to_vec(&info) {
                let _ = state.db.insert(AppState::make_key(PREFIX_INFO, &data_id), j);
            }
            let now = chrono::Utc::now().timestamp_micros();
            let mut trust = varcavia_uag::trust::TrustRecord::new(now);
            trust.attestations.push(varcavia_uag::trust::Attestation {
                source_pubkey: fact.source.clone(),
                domain: fact.domain.clone(),
                source_tier: varcavia_uag::trust::SourceTier::Website,
                timestamp_us: now,
            });
            trust.tier = varcavia_uag::trust::compute_tier(&trust, now);
            if let Ok(j) = serde_json::to_vec(&trust) {
                let _ = state.db.insert(AppState::make_key(PREFIX_TRUST, &data_id), j);
            }
            state.facts_ingested.fetch_add(1, Ordering::Relaxed);
            spider.stats.facts_new.fetch_add(1, Ordering::Relaxed);
            state.spider_new_facts.fetch_add(1, Ordering::Relaxed);
        }

        // Add new seeds from discovered concepts
        for (topic, domain) in result.new_seeds {
            if !spider.is_visited(&topic).await && topic.len() >= 3 {
                spider.add_seed(varcavia_crawler::spider::Seed {
                    topic, depth: seed.depth + 1, domain,
                }).await;
            }
        }

        cycle += 1;
        if cycle % 50 == 0 {
            tracing::info!(
                "Spider: {} topics, {} new facts, {} attested, {} contradictions, queue: {}",
                spider.stats.topics_crawled.load(Ordering::Relaxed),
                spider.stats.facts_new.load(Ordering::Relaxed),
                spider.stats.facts_attested.load(Ordering::Relaxed),
                spider.stats.contradictions_found.load(Ordering::Relaxed),
                spider.queue_size().await,
            );
            // Flush queue to sled for persistence across restarts
            spider.flush_queue(&state.db).await;
        }

        // Rate limit: 1 second between topics
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
