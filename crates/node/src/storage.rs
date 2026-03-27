//! Storage locale basato su sled (alternativa leggera a RocksDB).
//!
//! Wrapper KV store per dati + dDNA del nodo VARCAVIA.
//! Nota: il nodo attualmente usa AppState (uag/state.rs) per lo storage.
//! Questo modulo e preparato per uso futuro standalone.
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};

/// Wrapper attorno a sled per lo storage locale del nodo.
pub struct Storage {
    db: sled::Db,
}

/// Prefissi per separare i diversi tipi di dati nello store.
const PREFIX_DATA: &[u8] = b"d:";
const PREFIX_DDNA: &[u8] = b"n:";
const PREFIX_META: &[u8] = b"m:";

impl Storage {
    /// Apre (o crea) il database nella directory specificata.
    pub fn open(path: &str) -> Result<Self> {
        let expanded = shellexpand::tilde(path);
        let db = sled::open(expanded.as_ref())
            .with_context(|| format!("Impossibile aprire il database in: {path}"))?;
        tracing::info!("Storage aperto: {}", expanded);
        Ok(Storage { db })
    }

    /// Inserisce un dato raw con la sua chiave (blake3 hex).
    pub fn put_data(&self, key: &str, data: &[u8]) -> Result<()> {
        let full_key = make_key(PREFIX_DATA, key);
        self.db.insert(full_key, data)?;
        Ok(())
    }

    /// Recupera un dato raw per chiave.
    pub fn get_data(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let full_key = make_key(PREFIX_DATA, key);
        Ok(self.db.get(full_key)?.map(|v| v.to_vec()))
    }

    /// Inserisce un dDNA serializzato.
    pub fn put_ddna(&self, key: &str, ddna_bytes: &[u8]) -> Result<()> {
        let full_key = make_key(PREFIX_DDNA, key);
        self.db.insert(full_key, ddna_bytes)?;
        Ok(())
    }

    /// Recupera un dDNA serializzato.
    pub fn get_ddna(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let full_key = make_key(PREFIX_DDNA, key);
        Ok(self.db.get(full_key)?.map(|v| v.to_vec()))
    }

    /// Inserisce un valore JSON-serializzabile nei metadati.
    pub fn put_meta<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let full_key = make_key(PREFIX_META, key);
        let json = serde_json::to_vec(value)?;
        self.db.insert(full_key, json)?;
        Ok(())
    }

    /// Recupera un valore dai metadati.
    pub fn get_meta<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let full_key = make_key(PREFIX_META, key);
        match self.db.get(full_key)? {
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Elimina un dato (soft delete: marca come obsoleto).
    pub fn delete_data(&self, key: &str) -> Result<bool> {
        let full_key = make_key(PREFIX_DATA, key);
        Ok(self.db.remove(full_key)?.is_some())
    }

    /// Elenca tutte le chiavi dati presenti.
    pub fn list_data_keys(&self) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        for item in self.db.scan_prefix(PREFIX_DATA) {
            let (key, _) = item?;
            if let Ok(k) = std::str::from_utf8(&key[PREFIX_DATA.len()..]) {
                keys.push(k.to_string());
            }
        }
        Ok(keys)
    }

    /// Numero totale di entries nel database.
    pub fn count(&self) -> usize {
        self.db.len()
    }

    /// Forza il flush su disco.
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}

fn make_key(prefix: &[u8], key: &str) -> Vec<u8> {
    let mut full_key = Vec::with_capacity(prefix.len() + key.len());
    full_key.extend_from_slice(prefix);
    full_key.extend_from_slice(key.as_bytes());
    full_key
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_storage() -> Storage {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = format!("/tmp/varcavia-test-storage-{}-{}", std::process::id(), id);
        Storage::open(&path).unwrap()
    }

    #[test]
    fn test_put_get_data() {
        let storage = temp_storage();
        storage.put_data("key1", b"hello world").unwrap();
        let result = storage.get_data("key1").unwrap();
        assert_eq!(result, Some(b"hello world".to_vec()));
    }

    #[test]
    fn test_get_missing_key() {
        let storage = temp_storage();
        let result = storage.get_data("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_put_get_ddna() {
        let storage = temp_storage();
        let ddna_bytes = b"serialized ddna data";
        storage.put_ddna("ddna1", ddna_bytes).unwrap();
        let result = storage.get_ddna("ddna1").unwrap();
        assert_eq!(result, Some(ddna_bytes.to_vec()));
    }

    #[test]
    fn test_put_get_meta() {
        let storage = temp_storage();
        storage.put_meta("node_name", &"varcavia-01".to_string()).unwrap();
        let name: Option<String> = storage.get_meta("node_name").unwrap();
        assert_eq!(name, Some("varcavia-01".to_string()));
    }

    #[test]
    fn test_delete_data() {
        let storage = temp_storage();
        storage.put_data("to_delete", b"data").unwrap();
        assert!(storage.delete_data("to_delete").unwrap());
        assert!(storage.get_data("to_delete").unwrap().is_none());
    }

    #[test]
    fn test_list_data_keys() {
        let storage = temp_storage();
        storage.put_data("aaa", b"1").unwrap();
        storage.put_data("bbb", b"2").unwrap();
        let keys = storage.list_data_keys().unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"aaa".to_string()));
        assert!(keys.contains(&"bbb".to_string()));
    }
}
