//! Validazione fonte — Verifica firma Ed25519 e reputazione della fonte.

use varcavia_ddna::DataDna;
use crate::{CdeError, Result};

/// Soglia minima di reputazione della fonte (default da CLAUDE.md).
pub const MIN_SOURCE_REPUTATION: f32 = 0.3;

/// Risultato della validazione della fonte.
#[derive(Debug, Clone)]
pub struct SourceValidationResult {
    /// La firma è crittograficamente valida
    pub signature_valid: bool,
    /// Reputazione della fonte
    pub source_reputation: f32,
    /// La reputazione supera la soglia minima
    pub reputation_sufficient: bool,
}

/// Stadio 4: Valida la fonte del dato.
/// Verifica la firma Ed25519 e controlla la reputazione.
pub fn validate_source(
    data: &[u8],
    ddna: &DataDna,
    min_reputation: f32,
) -> Result<SourceValidationResult> {
    // Verifica firma crittografica
    let signature_valid = ddna.source.verify(&ddna.fingerprint).is_ok();

    if !signature_valid {
        return Err(CdeError::SourceValidationFailed(
            "Firma Ed25519 non valida".into(),
        ));
    }

    // Verifica che il contenuto corrisponda al fingerprint
    if ddna.verify_content(data).is_err() {
        return Err(CdeError::SourceValidationFailed(
            "Contenuto non corrisponde al fingerprint".into(),
        ));
    }

    let reputation_sufficient = ddna.source.reputation_score >= min_reputation;

    Ok(SourceValidationResult {
        signature_valid,
        source_reputation: ddna.source.reputation_score,
        reputation_sufficient,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use varcavia_ddna::identity::KeyPair;

    #[test]
    fn test_valid_source() {
        let kp = KeyPair::generate();
        let data = b"test data for validation";
        let ddna = DataDna::create(data, &kp).unwrap();
        let result = validate_source(data, &ddna, MIN_SOURCE_REPUTATION).unwrap();
        assert!(result.signature_valid);
        assert!(result.reputation_sufficient);
    }

    #[test]
    fn test_tampered_content_fails() {
        let kp = KeyPair::generate();
        let data = b"original data";
        let ddna = DataDna::create(data, &kp).unwrap();
        let result = validate_source(b"tampered", &ddna, MIN_SOURCE_REPUTATION);
        assert!(result.is_err());
    }

    #[test]
    fn test_low_reputation_flagged() {
        let kp = KeyPair::generate();
        let data = b"test data";
        let mut ddna = DataDna::create(data, &kp).unwrap();
        ddna.source.reputation_score = 0.1;
        // Bypass integrity check for test by not re-computing
        let result = validate_source(data, &ddna, MIN_SOURCE_REPUTATION).unwrap();
        assert!(!result.reputation_sufficient);
    }

    #[test]
    fn test_high_reputation_threshold() {
        let kp = KeyPair::generate();
        let data = b"test data";
        let ddna = DataDna::create(data, &kp).unwrap();
        let result = validate_source(data, &ddna, 0.9).unwrap();
        assert!(!result.reputation_sufficient); // default rep is 0.5
    }
}
