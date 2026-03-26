//! Content Fingerprint — Doppio hash BLAKE3 + SHA3-512 del contenuto.

use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_512};

/// Impronta digitale del contenuto di un dato.
/// Usa doppio hash per resilienza: se uno dei due algoritmi venisse
/// compromesso in futuro, l'altro mantiene la sicurezza.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentFingerprint {
    /// Hash BLAKE3 (32 bytes) — veloce, usato per lookup
    pub blake3: [u8; 32],
    /// Hash SHA3-512 (64 bytes) — sicuro, usato per verifica crittografica
    pub sha3_512: [u8; 64],
    /// Dimensione originale del contenuto in bytes
    pub content_size: u64,
}

impl ContentFingerprint {
    /// Calcola il fingerprint di un contenuto raw.
    pub fn compute(content: &[u8]) -> Self {
        // BLAKE3 — hash veloce per lookup e deduplicazione
        let blake3_hash = blake3::hash(content);

        // SHA3-512 — hash crittografico forte per verifica
        let mut sha3_hasher = Sha3_512::new();
        sha3_hasher.update(content);
        let sha3_result = sha3_hasher.finalize();

        let mut sha3_bytes = [0u8; 64];
        sha3_bytes.copy_from_slice(&sha3_result);

        ContentFingerprint {
            blake3: *blake3_hash.as_bytes(),
            sha3_512: sha3_bytes,
            content_size: content.len() as u64,
        }
    }

    /// Verifica che un contenuto corrisponda a questo fingerprint.
    pub fn matches(&self, content: &[u8]) -> bool {
        let computed = Self::compute(content);
        self.blake3 == computed.blake3 && self.sha3_512 == computed.sha3_512
    }

    /// Restituisce l'hash BLAKE3 come stringa esadecimale (usato come ID).
    pub fn id_hex(&self) -> String {
        hex::encode(self.blake3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        let data = b"test content for fingerprint";
        let fp1 = ContentFingerprint::compute(data);
        let fp2 = ContentFingerprint::compute(data);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_different_content() {
        let fp1 = ContentFingerprint::compute(b"content A");
        let fp2 = ContentFingerprint::compute(b"content B");
        assert_ne!(fp1.blake3, fp2.blake3);
        assert_ne!(fp1.sha3_512, fp2.sha3_512);
    }

    #[test]
    fn test_matches() {
        let data = b"verify me";
        let fp = ContentFingerprint::compute(data);
        assert!(fp.matches(data));
        assert!(!fp.matches(b"wrong data"));
    }

    #[test]
    fn test_content_size() {
        let data = b"12345";
        let fp = ContentFingerprint::compute(data);
        assert_eq!(fp.content_size, 5);
    }
}
