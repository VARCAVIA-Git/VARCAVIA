//! Chain of Custody — Registro immutabile dei passaggi di un dato.

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use chrono::Utc;
use crate::identity::KeyPair;
use crate::{DdnaError, Result};

/// Azione eseguita sul dato da un nodo.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CustodyAction {
    /// Il dato è stato creato da questo nodo
    Created,
    /// Il dato è stato ricevuto da un altro nodo
    Received,
    /// Il dato è stato validato da questo nodo
    Validated,
    /// Il dato è stato inoltrato ad un altro nodo
    Forwarded,
}

/// Singola entry nella catena di custodia.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyEntry {
    /// ID del nodo (chiave pubblica, 32 bytes)
    pub node_id: [u8; 32],
    /// Timestamp dell'azione in microsecondi
    pub timestamp_us: i64,
    /// Tipo di azione
    pub action: CustodyAction,
    /// Firma dell'entry da parte del nodo
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}

impl CustodyEntry {
    /// Crea una nuova entry di creazione (prima entry nella catena).
    pub fn new_creation(node_id: &[u8; 32], keypair: &KeyPair) -> Result<Self> {
        Self::new(node_id, CustodyAction::Created, keypair)
    }

    /// Crea una nuova entry nella catena di custodia.
    pub fn new(node_id: &[u8; 32], action: CustodyAction, keypair: &KeyPair) -> Result<Self> {
        let timestamp_us = Utc::now().timestamp_micros();

        // Messaggio da firmare: node_id + timestamp + action (serializzata)
        let mut message = Vec::new();
        message.extend_from_slice(node_id);
        message.extend_from_slice(&timestamp_us.to_le_bytes());
        let action_byte = match &action {
            CustodyAction::Created => 0u8,
            CustodyAction::Received => 1u8,
            CustodyAction::Validated => 2u8,
            CustodyAction::Forwarded => 3u8,
        };
        message.push(action_byte);

        let signature = keypair.sign(&message);

        Ok(CustodyEntry {
            node_id: *node_id,
            timestamp_us,
            action,
            signature,
        })
    }
}

/// Verifica l'intera catena di custodia.
pub fn verify_chain(chain: &[CustodyEntry]) -> Result<()> {
    if chain.is_empty() {
        return Err(DdnaError::InvalidCustody("Catena vuota".into()));
    }
    // La prima entry deve essere Created
    if chain[0].action != CustodyAction::Created {
        return Err(DdnaError::InvalidCustody(
            "Prima entry non è Created".into(),
        ));
    }
    // I timestamp devono essere non-decrescenti
    for window in chain.windows(2) {
        if window[1].timestamp_us < window[0].timestamp_us {
            return Err(DdnaError::InvalidCustody(
                "Timestamp non monotono nella catena".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_entry() {
        let kp = KeyPair::generate();
        let node_id = kp.public_key_bytes();
        let entry = CustodyEntry::new_creation(&node_id, &kp).unwrap();
        assert_eq!(entry.action, CustodyAction::Created);
    }

    #[test]
    fn test_verify_chain() {
        let kp = KeyPair::generate();
        let node_id = kp.public_key_bytes();
        let entry = CustodyEntry::new_creation(&node_id, &kp).unwrap();
        assert!(verify_chain(&[entry]).is_ok());
    }

    #[test]
    fn test_empty_chain_fails() {
        assert!(verify_chain(&[]).is_err());
    }

    #[test]
    fn test_non_created_first_entry_fails() {
        let kp = KeyPair::generate();
        let node_id = kp.public_key_bytes();
        let entry = CustodyEntry::new(&node_id, CustodyAction::Received, &kp).unwrap();
        assert!(verify_chain(&[entry]).is_err());
    }

    #[test]
    fn test_multi_entry_chain() {
        let kp = KeyPair::generate();
        let node_id = kp.public_key_bytes();
        let e1 = CustodyEntry::new_creation(&node_id, &kp).unwrap();
        let e2 = CustodyEntry::new(&node_id, CustodyAction::Validated, &kp).unwrap();
        assert!(verify_chain(&[e1, e2]).is_ok());
    }

    #[test]
    fn test_all_action_types() {
        let kp = KeyPair::generate();
        let node_id = kp.public_key_bytes();
        for action in [CustodyAction::Created, CustodyAction::Received,
                       CustodyAction::Validated, CustodyAction::Forwarded] {
            let entry = CustodyEntry::new(&node_id, action, &kp).unwrap();
            assert_eq!(entry.node_id, node_id);
        }
    }
}
