//! Codec — Serializzazione/deserializzazione dDNA in MessagePack.

use crate::{DataDna, DdnaError, Result};

/// Serializza un dDNA in formato MessagePack (compatto e veloce).
pub fn serialize(ddna: &DataDna) -> Result<Vec<u8>> {
    rmp_serde::to_vec(ddna)
        .map_err(|e| DdnaError::SerializationError(format!("Serializzazione fallita: {e}")))
}

/// Deserializza un dDNA da bytes MessagePack.
pub fn deserialize(data: &[u8]) -> Result<DataDna> {
    rmp_serde::from_slice(data)
        .map_err(|e| DdnaError::SerializationError(format!("Deserializzazione fallita: {e}")))
}

/// Serializza un dDNA in JSON (per debug e API).
pub fn to_json(ddna: &DataDna) -> Result<String> {
    serde_json::to_string_pretty(ddna)
        .map_err(|e| DdnaError::SerializationError(format!("JSON serialization failed: {e}")))
}

/// Deserializza un dDNA da JSON.
pub fn from_json(json: &str) -> Result<DataDna> {
    serde_json::from_str(json)
        .map_err(|e| DdnaError::SerializationError(format!("JSON deserialization failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    #[test]
    fn test_msgpack_roundtrip() {
        let kp = KeyPair::generate();
        let ddna = DataDna::create(b"test data", &kp).unwrap();
        let bytes = serialize(&ddna).unwrap();
        let restored = deserialize(&bytes).unwrap();
        assert_eq!(ddna.fingerprint.blake3, restored.fingerprint.blake3);
    }

    #[test]
    fn test_json_roundtrip() {
        let kp = KeyPair::generate();
        let ddna = DataDna::create(b"json test", &kp).unwrap();
        let json = to_json(&ddna).unwrap();
        let restored = from_json(&json).unwrap();
        assert_eq!(ddna.fingerprint.blake3, restored.fingerprint.blake3);
    }

    #[test]
    fn test_msgpack_compact() {
        let kp = KeyPair::generate();
        let ddna = DataDna::create(b"compactness test", &kp).unwrap();
        let msgpack = serialize(&ddna).unwrap();
        let json = to_json(&ddna).unwrap();
        // MessagePack deve essere più compatto di JSON
        assert!(msgpack.len() < json.len());
    }
}
