//! Configurazione del nodo VARCAVIA.
//! Nota: il nodo attualmente usa CLI args (clap). Questo modulo e preparato
//! per il caricamento config da file TOML in futuro.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node: NodeSection,
    pub network: NetworkSection,
    pub storage: StorageSection,
    pub arc: ArcSection,
    pub cde: CdeSection,
    pub ai: AiSection,
    pub api: ApiSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSection {
    pub name: String,
    pub data_dir: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSection {
    pub listen_addr: String,
    pub listen_port: u16,
    pub bootstrap_nodes: Vec<String>,
    pub max_peers: usize,
    pub mdns_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSection {
    pub engine: String,
    pub max_size_gb: u32,
    pub compression: String,
    pub cache_size_mb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcSection {
    pub committee_size: usize,
    pub confirmation_threshold: f64,
    pub validation_timeout_ms: u64,
    pub reputation_decay_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdeSection {
    pub dedup_lsh_threshold: f64,
    pub semantic_dedup_threshold: f64,
    pub freshness_window_hours: u32,
    pub min_source_reputation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSection {
    pub onnx_model_path: String,
    pub embedding_dimensions: u32,
    pub max_batch_size: u32,
    pub agent_check_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSection {
    pub enabled: bool,
    pub bind_addr: String,
    pub cors_origins: Vec<String>,
    pub rate_limit_per_sec: u32,
}

impl NodeConfig {
    /// Carica la configurazione da un file TOML.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: NodeConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
