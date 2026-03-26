//! Normalizzazione — Conversione in VARCAVIA Universal Format (VUF).

use serde::{Deserialize, Serialize};
use crate::{CdeError, Result};

/// Schema supportati per la normalizzazione.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VufSchema {
    /// Dati JSON generici
    Json,
    /// Testo non strutturato
    PlainText,
    /// Dati tabulari (CSV-like)
    Tabular,
    /// Dato binario opaco
    Binary,
}

/// VARCAVIA Universal Format — formato normalizzato interno.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VufRecord {
    /// Schema del payload
    pub schema: VufSchema,
    /// Dominio del dato (es. "climate", "health", "finance")
    pub domain: String,
    /// Payload compresso con zstd
    pub payload: Vec<u8>,
    /// Dimensione originale prima della compressione
    pub original_size: u64,
}

/// Stadio 5: Normalizza un dato raw in formato VUF.
pub fn normalize(content: &[u8], domain: &str) -> Result<VufRecord> {
    let schema = detect_schema(content);
    let original_size = content.len() as u64;

    // Comprimi con zstd livello 3
    let payload = zstd::encode_all(content, 3)
        .map_err(|e| CdeError::NormalizationFailed(format!("Compressione fallita: {e}")))?;

    Ok(VufRecord {
        schema,
        domain: domain.to_string(),
        payload,
        original_size,
    })
}

/// Decomprime un VufRecord per ottenere il contenuto originale.
pub fn denormalize(record: &VufRecord) -> Result<Vec<u8>> {
    zstd::decode_all(record.payload.as_slice())
        .map_err(|e| CdeError::NormalizationFailed(format!("Decompressione fallita: {e}")))
}

/// Rileva automaticamente lo schema del contenuto.
fn detect_schema(content: &[u8]) -> VufSchema {
    // Prova a parsare come JSON
    if serde_json::from_slice::<serde_json::Value>(content).is_ok() {
        return VufSchema::Json;
    }

    // Controlla se è testo UTF-8 valido
    if let Ok(text) = std::str::from_utf8(content) {
        // Euristica per CSV: contiene virgole e newline regolari
        let lines: Vec<&str> = text.lines().collect();
        if lines.len() > 1 {
            let comma_counts: Vec<usize> = lines.iter().map(|l| l.matches(',').count()).collect();
            if comma_counts.iter().all(|&c| c > 0) && comma_counts.windows(2).all(|w| w[0] == w[1]) {
                return VufSchema::Tabular;
            }
        }
        return VufSchema::PlainText;
    }

    VufSchema::Binary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_json() {
        let json = br#"{"temp": 22, "city": "Roma"}"#;
        let record = normalize(json, "climate").unwrap();
        assert_eq!(record.schema, VufSchema::Json);
        assert_eq!(record.domain, "climate");
        assert_eq!(record.original_size, json.len() as u64);
    }

    #[test]
    fn test_normalize_text() {
        let text = b"La temperatura a Roma e' 22 gradi";
        let record = normalize(text, "climate").unwrap();
        assert_eq!(record.schema, VufSchema::PlainText);
    }

    #[test]
    fn test_normalize_csv() {
        let csv = b"city,temp,humidity\nRoma,22,65\nMilano,18,70\n";
        let record = normalize(csv, "climate").unwrap();
        assert_eq!(record.schema, VufSchema::Tabular);
    }

    #[test]
    fn test_roundtrip() {
        let original = b"test data for normalization roundtrip";
        let record = normalize(original, "test").unwrap();
        let restored = denormalize(&record).unwrap();
        assert_eq!(original.to_vec(), restored);
    }

    #[test]
    fn test_compression_reduces_size() {
        let big_data = "repeated content for compression test ".repeat(100);
        let record = normalize(big_data.as_bytes(), "test").unwrap();
        assert!(record.payload.len() < record.original_size as usize);
    }

    #[test]
    fn test_binary_detection() {
        let binary = vec![0xFF, 0xFE, 0x00, 0x80, 0x90, 0xAB];
        let record = normalize(&binary, "binary").unwrap();
        assert_eq!(record.schema, VufSchema::Binary);
    }
}
