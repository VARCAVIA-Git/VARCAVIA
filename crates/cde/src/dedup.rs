//! Deduplicazione — Hash esatto + LSH + Semantica (n-gram similarity).

use std::collections::{HashMap, HashSet};
use crate::{CdeError, Result};

/// Indice di deduplicazione basato su hash esatto.
#[derive(Debug, Default)]
pub struct ExactDedupIndex {
    known_hashes: HashMap<[u8; 32], String>,
}

impl ExactDedupIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(&self, blake3_hash: &[u8; 32]) -> Option<&str> {
        self.known_hashes.get(blake3_hash).map(|s| s.as_str())
    }

    pub fn insert(&mut self, blake3_hash: [u8; 32], data_id: String) {
        self.known_hashes.insert(blake3_hash, data_id);
    }

    pub fn len(&self) -> usize {
        self.known_hashes.len()
    }

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
#[derive(Debug)]
pub struct LshIndex {
    num_hashes: usize,
    threshold: f64,
    signatures: HashMap<String, Vec<u32>>,
}

impl LshIndex {
    pub fn new(num_hashes: usize, threshold: f64) -> Self {
        LshIndex {
            num_hashes,
            threshold,
            signatures: HashMap::new(),
        }
    }

    pub fn compute_signature(&self, content: &[u8]) -> Vec<u32> {
        let mut signature = vec![u32::MAX; self.num_hashes];
        if content.len() < 4 {
            return signature;
        }
        for window in content.windows(4) {
            let shingle = u32::from_le_bytes([window[0], window[1], window[2], window[3]]);
            for (i, min_val) in signature.iter_mut().enumerate() {
                let h = hash_with_seed(shingle, i as u32);
                if h < *min_val {
                    *min_val = h;
                }
            }
        }
        signature
    }

    pub fn estimated_similarity(sig_a: &[u32], sig_b: &[u32]) -> f64 {
        if sig_a.len() != sig_b.len() || sig_a.is_empty() {
            return 0.0;
        }
        let matches = sig_a.iter().zip(sig_b.iter()).filter(|(a, b)| a == b).count();
        matches as f64 / sig_a.len() as f64
    }

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

    pub fn insert(&mut self, data_id: String, content: &[u8]) {
        let sig = self.compute_signature(content);
        self.signatures.insert(data_id, sig);
    }

    pub fn len(&self) -> usize {
        self.signatures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }
}

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

// ============================================================
// Stadio 3: Deduplicazione semantica (character n-gram Jaccard)
// ============================================================
// Approccio Rust-native: confronto basato su trigrammi di caratteri.
// TODO: sostituire con embedding ONNX (all-MiniLM-L6-v2) nella Fase 5
// per catturare similarità semantica reale, non solo testuale.

/// Indice di deduplicazione semantica basato su character trigram Jaccard.
#[derive(Debug)]
pub struct SemanticDedupIndex {
    /// Soglia di similarità (0.0 - 1.0, default 0.9)
    threshold: f64,
    /// Trigrammi per ogni dato: data_id → set di trigrammi
    trigrams: HashMap<String, HashSet<String>>,
}

impl SemanticDedupIndex {
    pub fn new(threshold: f64) -> Self {
        SemanticDedupIndex {
            threshold,
            trigrams: HashMap::new(),
        }
    }

    /// Estrae i trigrammi di caratteri da un testo (normalizzato in lowercase).
    pub fn extract_trigrams(text: &str) -> HashSet<String> {
        let normalized: String = text
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect();
        let chars: Vec<char> = normalized.chars().collect();
        let mut trigrams = HashSet::new();
        if chars.len() >= 3 {
            for window in chars.windows(3) {
                trigrams.insert(window.iter().collect());
            }
        }
        trigrams
    }

    /// Calcola la similarità Jaccard tra due set di trigrammi.
    pub fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        let intersection = a.intersection(b).count();
        let union = a.union(b).count();
        if union == 0 {
            return 0.0;
        }
        intersection as f64 / union as f64
    }

    /// Controlla se un testo è un duplicato semantico.
    /// Restituisce l'ID e la similarità del match più vicino.
    pub fn check_semantic_duplicate(&self, text: &str) -> Option<(String, f64)> {
        let query_trigrams = Self::extract_trigrams(text);
        let mut best: Option<(String, f64)> = None;
        for (id, existing) in &self.trigrams {
            let sim = Self::jaccard_similarity(&query_trigrams, existing);
            if sim >= self.threshold
                && best.as_ref().map_or(true, |(_, best_sim)| sim > *best_sim)
            {
                best = Some((id.clone(), sim));
            }
        }
        best
    }

    /// Inserisce un testo nell'indice.
    pub fn insert(&mut self, data_id: String, text: &str) {
        let trigrams = Self::extract_trigrams(text);
        self.trigrams.insert(data_id, trigrams);
    }

    pub fn len(&self) -> usize {
        self.trigrams.len()
    }

    pub fn is_empty(&self) -> bool {
        self.trigrams.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ExactDedupIndex ---

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

    // --- LshIndex ---

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
        let sig = lsh.compute_signature(b"ab");
        assert_eq!(sig.len(), 64);
    }

    // --- SemanticDedupIndex ---

    #[test]
    fn test_semantic_identical_text() {
        let mut idx = SemanticDedupIndex::new(0.8);
        idx.insert("doc-1".into(), "The temperature in Rome is 22 degrees");
        let result = idx.check_semantic_duplicate("The temperature in Rome is 22 degrees");
        assert!(result.is_some());
        let (_, sim) = result.unwrap();
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_semantic_similar_text() {
        let mut idx = SemanticDedupIndex::new(0.6);
        idx.insert("doc-1".into(), "The temperature in Rome is 22 degrees celsius today");
        let result = idx.check_semantic_duplicate(
            "The temperature in Rome is 23 degrees celsius today"
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_semantic_different_text() {
        let mut idx = SemanticDedupIndex::new(0.5);
        idx.insert("doc-1".into(), "The temperature in Rome is 22 degrees");
        let result = idx.check_semantic_duplicate(
            "Stock market closes at record high today"
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_trigram_extraction() {
        let trigrams = SemanticDedupIndex::extract_trigrams("hello");
        assert!(trigrams.contains("hel"));
        assert!(trigrams.contains("ell"));
        assert!(trigrams.contains("llo"));
        assert_eq!(trigrams.len(), 3);
    }

    #[test]
    fn test_jaccard_identical() {
        let a = SemanticDedupIndex::extract_trigrams("test data");
        let b = SemanticDedupIndex::extract_trigrams("test data");
        let sim = SemanticDedupIndex::jaccard_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_semantic_case_insensitive() {
        let mut idx = SemanticDedupIndex::new(0.8);
        idx.insert("doc-1".into(), "HELLO WORLD");
        let result = idx.check_semantic_duplicate("hello world");
        assert!(result.is_some());
    }

    #[test]
    fn test_semantic_short_text() {
        let mut idx = SemanticDedupIndex::new(0.5);
        idx.insert("doc-1".into(), "hi");
        // Very short text has few trigrams
        let result = idx.check_semantic_duplicate("hi");
        // Both have 0 trigrams (< 3 chars), jaccard returns 1.0
        assert!(result.is_some());
    }
}
