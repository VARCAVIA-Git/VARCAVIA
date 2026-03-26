//! Deduplicazione — Hash esatto + LSH per near-duplicates.

use std::collections::HashMap;
use crate::{CdeError, Result};

/// Indice di deduplicazione basato su hash esatto.
#[derive(Debug, Default)]
pub struct ExactDedupIndex {
    /// Mappa blake3_hash → ID del dato esistente
    known_hashes: HashMap<[u8; 32], String>,
}

impl ExactDedupIndex {
    /// Crea un nuovo indice vuoto.
    pub fn new() -> Self {
        Self::default()
    }

    /// Controlla se un hash esatto esiste già.
    /// Restituisce l'ID del dato esistente se trovato.
    pub fn check(&self, blake3_hash: &[u8; 32]) -> Option<&str> {
        self.known_hashes.get(blake3_hash).map(|s| s.as_str())
    }

    /// Registra un nuovo hash nell'indice.
    pub fn insert(&mut self, blake3_hash: [u8; 32], data_id: String) {
        self.known_hashes.insert(blake3_hash, data_id);
    }

    /// Numero di hash nell'indice.
    pub fn len(&self) -> usize {
        self.known_hashes.len()
    }

    /// Verifica se l'indice è vuoto.
    pub fn is_empty(&self) -> bool {
        self.known_hashes.is_empty()
    }
}

/// Risultato dello Stadio 1: deduplicazione hash esatto.
pub fn check_exact_duplicate(
    blake3_hash: &[u8; 32],
    index: &ExactDedupIndex,
) -> Result<Option<String>> {
    if let Some(existing_id) = index.check(blake3_hash) {
        Err(CdeError::DuplicateData(format!(
            "Duplicato esatto trovato: {}",
            existing_id
        )))
    } else {
        Ok(None)
    }
}

/// Indice LSH (Locality-Sensitive Hashing) per near-duplicates.
/// Usa MinHash semplificato per stimare la similarità Jaccard.
#[derive(Debug)]
pub struct LshIndex {
    /// Numero di funzioni hash per la firma MinHash
    num_hashes: usize,
    /// Soglia di similarità per segnalare near-duplicate
    threshold: f64,
    /// Firme MinHash dei dati inseriti: data_id → firma
    signatures: HashMap<String, Vec<u32>>,
}

impl LshIndex {
    /// Crea un nuovo indice LSH.
    pub fn new(num_hashes: usize, threshold: f64) -> Self {
        LshIndex {
            num_hashes,
            threshold,
            signatures: HashMap::new(),
        }
    }

    /// Calcola la firma MinHash di un contenuto.
    pub fn compute_signature(&self, content: &[u8]) -> Vec<u32> {
        let mut signature = vec![u32::MAX; self.num_hashes];

        // Genera shingles (n-grammi di bytes, n=4)
        if content.len() < 4 {
            return signature;
        }

        for window in content.windows(4) {
            let shingle = u32::from_le_bytes([window[0], window[1], window[2], window[3]]);
            for (i, min_val) in signature.iter_mut().enumerate() {
                // Hash con seed diverso per ogni funzione
                let h = hash_with_seed(shingle, i as u32);
                if h < *min_val {
                    *min_val = h;
                }
            }
        }

        signature
    }

    /// Calcola la similarità Jaccard stimata tra due firme MinHash.
    pub fn estimated_similarity(sig_a: &[u32], sig_b: &[u32]) -> f64 {
        if sig_a.len() != sig_b.len() || sig_a.is_empty() {
            return 0.0;
        }
        let matches = sig_a.iter().zip(sig_b.iter()).filter(|(a, b)| a == b).count();
        matches as f64 / sig_a.len() as f64
    }

    /// Controlla se un contenuto è un near-duplicate di qualcosa nell'indice.
    /// Restituisce l'ID e la similarità del match più vicino.
    pub fn check_near_duplicate(&self, content: &[u8]) -> Option<(String, f64)> {
        let sig = self.compute_signature(content);
        let mut best_match: Option<(String, f64)> = None;

        for (id, existing_sig) in &self.signatures {
            let sim = Self::estimated_similarity(&sig, existing_sig);
            if sim >= self.threshold
                && best_match.as_ref().map_or(true, |(_, best_sim)| sim > *best_sim)
            {
                best_match = Some((id.clone(), sim));
            }
        }

        best_match
    }

    /// Inserisce un contenuto nell'indice.
    pub fn insert(&mut self, data_id: String, content: &[u8]) {
        let sig = self.compute_signature(content);
        self.signatures.insert(data_id, sig);
    }

    /// Numero di entries nell'indice.
    pub fn len(&self) -> usize {
        self.signatures.len()
    }

    /// Verifica se l'indice è vuoto.
    pub fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }
}

/// Hash semplice con seed per MinHash.
fn hash_with_seed(value: u32, seed: u32) -> u32 {
    let mut h = value.wrapping_mul(0x9E3779B9);
    h = h.wrapping_add(seed.wrapping_mul(0x517CC1B7));
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EBCA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2AE35);
    h ^= h >> 16;
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_dedup_empty() {
        let index = ExactDedupIndex::new();
        assert!(index.is_empty());
        let hash = [0u8; 32];
        assert!(check_exact_duplicate(&hash, &index).unwrap().is_none());
    }

    #[test]
    fn test_exact_dedup_found() {
        let mut index = ExactDedupIndex::new();
        let hash = *blake3::hash(b"test data").as_bytes();
        index.insert(hash, "data-001".into());
        assert!(check_exact_duplicate(&hash, &index).is_err());
    }

    #[test]
    fn test_exact_dedup_not_found() {
        let mut index = ExactDedupIndex::new();
        let hash1 = *blake3::hash(b"data A").as_bytes();
        let hash2 = *blake3::hash(b"data B").as_bytes();
        index.insert(hash1, "data-001".into());
        assert!(check_exact_duplicate(&hash2, &index).unwrap().is_none());
    }

    #[test]
    fn test_lsh_identical_content() {
        let mut lsh = LshIndex::new(64, 0.85);
        let content = b"This is a test document for LSH deduplication testing purposes";
        lsh.insert("doc-1".into(), content);
        let result = lsh.check_near_duplicate(content);
        assert!(result.is_some());
        let (_, sim) = result.unwrap();
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_lsh_different_content() {
        let mut lsh = LshIndex::new(64, 0.85);
        lsh.insert("doc-1".into(), b"completely different document about cats and dogs");
        let result = lsh.check_near_duplicate(
            b"another unrelated text about mathematics and physics"
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_lsh_similar_content() {
        let mut lsh = LshIndex::new(128, 0.5);
        let original = b"The temperature in Rome is 22 degrees celsius today";
        let similar = b"The temperature in Rome is 23 degrees celsius today";
        lsh.insert("doc-1".into(), original);
        let result = lsh.check_near_duplicate(similar);
        // Similar content should have high similarity
        assert!(result.is_some());
    }

    #[test]
    fn test_minhash_deterministic() {
        let lsh = LshIndex::new(64, 0.85);
        let content = b"deterministic test content";
        let sig1 = lsh.compute_signature(content);
        let sig2 = lsh.compute_signature(content);
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_lsh_short_content() {
        let lsh = LshIndex::new(64, 0.85);
        let sig = lsh.compute_signature(b"ab"); // shorter than shingle size
        assert_eq!(sig.len(), 64);
    }
}
