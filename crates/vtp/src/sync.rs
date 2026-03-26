//! Store-and-Forward + CRDT synchronization.
//!
//! Gestisce la sincronizzazione quando i nodi sono temporaneamente disconnessi.
//! Usa CRDT (Conflict-free Replicated Data Types) per convergenza senza conflitti.
//!
//! TODO (Fase 2): implementare G-Counter e LWW-Register per stato nodo.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Registro Last-Writer-Wins (LWW) per un singolo valore.
/// Convergenza automatica: il valore con timestamp più recente vince.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LwwRegister<T: Clone> {
    pub value: T,
    pub timestamp_us: i64,
    pub node_id: [u8; 32],
}

impl<T: Clone> LwwRegister<T> {
    /// Crea un nuovo registro.
    pub fn new(value: T, node_id: [u8; 32]) -> Self {
        LwwRegister {
            value,
            timestamp_us: chrono::Utc::now().timestamp_micros(),
            node_id,
        }
    }

    /// Merge con un altro registro: il timestamp più recente vince.
    pub fn merge(&mut self, other: &LwwRegister<T>) {
        if other.timestamp_us > self.timestamp_us {
            self.value = other.value.clone();
            self.timestamp_us = other.timestamp_us;
            self.node_id = other.node_id;
        }
    }
}

/// G-Set (Grow-only Set) — insieme a cui si possono solo aggiungere elementi.
/// Usato per tracciare quali dDNA sono stati visti da un nodo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GSet {
    pub elements: HashMap<[u8; 32], i64>, // hash → timestamp
}

impl GSet {
    pub fn new() -> Self {
        GSet { elements: HashMap::new() }
    }

    pub fn insert(&mut self, hash: [u8; 32]) {
        self.elements.entry(hash).or_insert_with(|| {
            chrono::Utc::now().timestamp_micros()
        });
    }

    pub fn contains(&self, hash: &[u8; 32]) -> bool {
        self.elements.contains_key(hash)
    }

    /// Merge: unione degli insiemi.
    pub fn merge(&mut self, other: &GSet) {
        for (k, v) in &other.elements {
            self.elements.entry(*k).or_insert(*v);
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

impl Default for GSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lww_merge() {
        let mut r1 = LwwRegister::new("old".to_string(), [1u8; 32]);
        std::thread::sleep(std::time::Duration::from_millis(1));
        let r2 = LwwRegister::new("new".to_string(), [2u8; 32]);
        r1.merge(&r2);
        assert_eq!(r1.value, "new");
    }

    #[test]
    fn test_gset_merge() {
        let mut s1 = GSet::new();
        let mut s2 = GSet::new();
        s1.insert([1u8; 32]);
        s2.insert([2u8; 32]);
        s1.merge(&s2);
        assert_eq!(s1.len(), 2);
    }
}
