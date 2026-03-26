//! Struttura pacchetti VTP con header semantico.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::priority::SemanticPriority;

/// Pacchetto VTP — unità di trasporto nel protocollo VARCAVIA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VtpPacket {
    /// ID univoco del pacchetto
    pub id: Uuid,
    /// Versione del protocollo
    pub version: u8,
    /// Priorità semantica del payload
    pub priority: SemanticPriority,
    /// ID del nodo sorgente
    pub source_node: [u8; 32],
    /// ID del nodo destinazione (o broadcast)
    pub dest_node: Option<[u8; 32]>,
    /// Timestamp di creazione (microsecondi)
    pub created_at_us: i64,
    /// TTL (hop rimanenti)
    pub ttl: u8,
    /// Il payload è compresso con delta compression?
    pub is_delta: bool,
    /// Hash del payload per verifica integrità
    pub payload_hash: [u8; 32],
    /// Payload (dato + dDNA serializzati, possibilmente compressi)
    pub payload: Vec<u8>,
}

impl VtpPacket {
    /// Crea un nuovo pacchetto VTP.
    pub fn new(
        source_node: [u8; 32],
        dest_node: Option<[u8; 32]>,
        priority: SemanticPriority,
        payload: Vec<u8>,
    ) -> Self {
        let payload_hash = *blake3::hash(&payload).as_bytes();
        VtpPacket {
            id: Uuid::new_v4(),
            version: 1,
            priority,
            source_node,
            dest_node,
            created_at_us: chrono::Utc::now().timestamp_micros(),
            ttl: 16,
            is_delta: false,
            payload_hash,
            payload,
        }
    }

    /// Dimensione totale del pacchetto in bytes.
    pub fn size(&self) -> usize {
        // Header fisso ~128 bytes + payload
        128 + self.payload.len()
    }

    /// Verifica integrità del payload.
    pub fn verify_payload(&self) -> bool {
        let hash = *blake3::hash(&self.payload).as_bytes();
        hash == self.payload_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_packet() {
        let pkt = VtpPacket::new(
            [1u8; 32],
            None,
            SemanticPriority::Normal,
            b"test payload".to_vec(),
        );
        assert!(pkt.verify_payload());
        assert_eq!(pkt.version, 1);
        assert_eq!(pkt.ttl, 16);
    }

    #[test]
    fn test_corrupted_payload() {
        let mut pkt = VtpPacket::new(
            [1u8; 32],
            None,
            SemanticPriority::Normal,
            b"original".to_vec(),
        );
        pkt.payload = b"tampered".to_vec();
        assert!(!pkt.verify_payload());
    }
}
