//! Delta Compression — Trasmette solo le differenze tra versioni.

use crate::{VtpError, Result};

/// Comprime dati con zstd (livello 3, buon compromesso velocità/compressione).
pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
    zstd::encode_all(data, 3)
        .map_err(|e| VtpError::CompressionError(format!("Compressione fallita: {e}")))
}

/// Decomprime dati zstd.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    zstd::decode_all(data)
        .map_err(|e| VtpError::CompressionError(format!("Decompressione fallita: {e}")))
}

/// Calcola il delta tra una versione vecchia e una nuova.
/// Restituisce solo le differenze (operazioni di patch).
/// TODO (Fase 2): implementare delta binario avanzato (simile a rsync).
pub fn compute_delta(old: &[u8], new: &[u8]) -> Result<Vec<u8>> {
    // Per ora: semplice XOR-based delta per dati della stessa lunghezza,
    // fallback a compressione completa per lunghezze diverse.
    if old.len() == new.len() {
        let delta: Vec<u8> = old.iter().zip(new.iter()).map(|(a, b)| a ^ b).collect();
        compress(&delta)
    } else {
        compress(new)
    }
}

/// Applica un delta a una versione base per ottenere la versione nuova.
pub fn apply_delta(base: &[u8], delta_compressed: &[u8]) -> Result<Vec<u8>> {
    let delta = decompress(delta_compressed)?;
    if base.len() == delta.len() {
        Ok(base.iter().zip(delta.iter()).map(|(a, b)| a ^ b).collect())
    } else {
        // Delta è la versione completa compressa
        Ok(delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress() {
        let data = b"test data for compression that should be compressed well if repeated repeated repeated";
        let compressed = compress(data).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(data.to_vec(), decompressed);
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_delta_same_length() {
        let old = b"hello world AAAA";
        let new = b"hello world BBBB";
        let delta = compute_delta(old, new).unwrap();
        let restored = apply_delta(old, &delta).unwrap();
        assert_eq!(new.to_vec(), restored);
    }
}
