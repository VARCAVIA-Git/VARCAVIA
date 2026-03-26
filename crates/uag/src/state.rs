//! Stato condiviso dell'applicazione — passato a tutti gli handler Axum.

use std::net::SocketAddr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::RwLock;
use varcavia_cde::pipeline::{Pipeline, PipelineConfig};
use varcavia_ddna::identity::KeyPair;

/// Prefissi per separare i tipi di dati in sled.
pub const PREFIX_DATA: &[u8] = b"d:";
pub const PREFIX_DDNA: &[u8] = b"n:";
pub const PREFIX_INFO: &[u8] = b"i:";
pub const PREFIX_CONSENSUS: &[u8] = b"c:";

/// Stato condiviso tra tutti gli handler REST.
pub struct AppState {
    /// Database sled embedded
    pub db: sled::Db,
    /// Pipeline CDE (richiede &mut self, protetta da Mutex)
    pub pipeline: Mutex<Pipeline>,
    /// Chiave segreta del nodo (32 bytes) — usata per creare dDNA
    pub node_secret: [u8; 32],
    /// ID del nodo (hex della chiave pubblica)
    pub node_id: String,
    /// Timestamp di avvio del nodo
    pub started_at: Instant,
    /// Indirizzi P2P dei peer noti
    pub peer_addrs: RwLock<Vec<SocketAddr>>,
    /// Contatore totale verifiche (atomico, lock-free)
    pub total_verifications: AtomicU64,
    /// Contatore fatti inseriti dall'avvio (atomico)
    pub facts_ingested: AtomicU64,
    /// Somma latenze richieste in microsecondi (per calcolo media)
    pub latency_sum_us: AtomicU64,
    /// Contatore richieste per media latenza
    pub latency_count: AtomicU64,
}

impl AppState {
    /// Crea un nuovo AppState.
    pub fn new(db: sled::Db, node_secret: [u8; 32], pipeline_config: PipelineConfig) -> Self {
        let keypair = KeyPair::from_bytes(&node_secret);
        let node_id = hex::encode(keypair.public_key_bytes());
        AppState {
            db,
            pipeline: Mutex::new(Pipeline::new(pipeline_config)),
            node_secret,
            node_id,
            started_at: Instant::now(),
            peer_addrs: RwLock::new(Vec::new()),
            total_verifications: AtomicU64::new(0),
            facts_ingested: AtomicU64::new(0),
            latency_sum_us: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
        }
    }

    /// Restituisce il KeyPair del nodo.
    pub fn keypair(&self) -> KeyPair {
        KeyPair::from_bytes(&self.node_secret)
    }

    /// Uptime del nodo in secondi.
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    /// Conta i dati nel database (scan prefix "d:").
    pub fn data_count(&self) -> u64 {
        self.db.scan_prefix(PREFIX_DATA).count() as u64
    }

    /// Incrementa contatore verifiche e restituisce il nuovo valore.
    pub fn inc_verifications(&self) -> u64 {
        self.total_verifications.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Helper: costruisce una chiave con prefisso.
    pub fn make_key(prefix: &[u8], id: &str) -> Vec<u8> {
        let mut key = Vec::with_capacity(prefix.len() + id.len());
        key.extend_from_slice(prefix);
        key.extend_from_slice(id.as_bytes());
        key
    }

    /// Registra la latenza di una richiesta e restituisce la media in ms.
    pub fn record_latency_us(&self, us: u64) {
        self.latency_sum_us.fetch_add(us, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Media latenza in millisecondi.
    pub fn avg_latency_ms(&self) -> f64 {
        let count = self.latency_count.load(Ordering::Relaxed);
        if count == 0 { return 0.0; }
        let sum = self.latency_sum_us.load(Ordering::Relaxed);
        (sum as f64 / count as f64) / 1000.0
    }

    /// Aggiunge un peer address.
    pub async fn add_peer(&self, addr: SocketAddr) {
        let mut peers = self.peer_addrs.write().await;
        if !peers.contains(&addr) {
            peers.push(addr);
        }
    }

    /// Restituisce la lista peer corrente.
    pub async fn get_peers(&self) -> Vec<SocketAddr> {
        self.peer_addrs.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_state() -> AppState {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let kp = KeyPair::generate();
        AppState::new(db, kp.secret_bytes(), PipelineConfig::default())
    }

    #[test]
    fn test_state_creation() {
        let state = temp_state();
        assert!(!state.node_id.is_empty());
        assert_eq!(state.data_count(), 0);
    }

    #[test]
    fn test_keypair_consistency() {
        let state = temp_state();
        let kp = state.keypair();
        assert_eq!(hex::encode(kp.public_key_bytes()), state.node_id);
    }

    #[test]
    fn test_make_key() {
        let key = AppState::make_key(PREFIX_DATA, "abc123");
        assert_eq!(&key[..2], b"d:");
        assert_eq!(&key[2..], b"abc123");
    }

    #[test]
    fn test_uptime() {
        let state = temp_state();
        assert!(state.uptime_secs() < 2);
    }

    #[test]
    fn test_verifications_counter() {
        let state = temp_state();
        assert_eq!(state.inc_verifications(), 1);
        assert_eq!(state.inc_verifications(), 2);
        assert_eq!(state.total_verifications.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn test_add_peer() {
        let state = temp_state();
        let addr: SocketAddr = "127.0.0.1:8181".parse().unwrap();
        state.add_peer(addr).await;
        let peers = state.get_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0], addr);
    }

    #[tokio::test]
    async fn test_add_peer_dedup() {
        let state = temp_state();
        let addr: SocketAddr = "127.0.0.1:8181".parse().unwrap();
        state.add_peer(addr).await;
        state.add_peer(addr).await;
        assert_eq!(state.get_peers().await.len(), 1);
    }
}
