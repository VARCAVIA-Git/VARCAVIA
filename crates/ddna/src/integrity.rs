//! Integrity Proof — Hash dell'intero dDNA per auto-verifica.

use crate::{DdnaError, Result};

/// Calcola l'hash di integrità dell'intero dDNA.
/// Usa BLAKE3 su tutti i campi del dDNA serializzati (escluso integrity_hash stesso).
pub fn compute_integrity_hash(ddna: &super::DataDna) -> Result<[u8; 32]> {
    let mut hasher = blake3::Hasher::new();

    // Version
    hasher.update(&[ddna.version]);

    // Fingerprint
    hasher.update(&ddna.fingerprint.blake3);
    hasher.update(&ddna.fingerprint.sha3_512);
    hasher.update(&ddna.fingerprint.content_size.to_le_bytes());

    // Source Identity
    hasher.update(&ddna.source.public_key);
    hasher.update(&ddna.source.signature);
    hasher.update(&ddna.source.reputation_score.to_le_bytes());

    // Temporal Proof
    hasher.update(&ddna.temporal.timestamp_us.to_le_bytes());
    hasher.update(&ddna.temporal.precision_us.to_le_bytes());

    // Custody Chain
    for entry in &ddna.custody_chain {
        hasher.update(&entry.node_id);
        hasher.update(&entry.timestamp_us.to_le_bytes());
        hasher.update(&entry.signature);
    }

    // Semantic Vector (se presente)
    if let Some(ref sv) = ddna.semantic_vector {
        hasher.update(sv.model_id.as_bytes());
        hasher.update(&sv.dimensions.to_le_bytes());
        for v in &sv.values {
            hasher.update(&v.to_le_bytes());
        }
    }

    Ok(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    #[test]
    fn test_integrity_deterministic() {
        let kp = KeyPair::generate();
        let ddna = crate::DataDna::create(b"test", &kp).unwrap();
        let hash1 = compute_integrity_hash(&ddna).unwrap();
        let hash2 = compute_integrity_hash(&ddna).unwrap();
        assert_eq!(hash1, hash2);
    }
}
