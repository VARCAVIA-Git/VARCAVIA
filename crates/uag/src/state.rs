//! Stato condiviso dell'applicazione — passato a tutti gli handler Axum.

use std::sync::Mutex;
use std::time::Instant;
use varcavia_cde::pipeline::{Pipeline, PipelineConfig};
use varcavia_ddna::identity::KeyPair;

/// Prefissi per separare i tipi di dati in sled.
pub const PREFIX_DATA: &[u8] = b"d:";
pub const PREFIX_DDNA: &[u8] = b"n:";
pub const PREFIX_INFO: &[u8] = b"i:";

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

    /// Helper: costruisce una chiave con prefisso.
    pub fn make_key(prefix: &[u8], id: &str) -> Vec<u8> {
        let mut key = Vec::with_capacity(prefix.len() + id.len());
        key.extend_from_slice(prefix);
        key.extend_from_slice(id.as_bytes());
        key
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
}
