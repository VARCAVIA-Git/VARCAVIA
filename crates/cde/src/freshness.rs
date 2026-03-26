//! Freshness — Controllo freschezza temporale dei dati.

use chrono::Utc;
use varcavia_ddna::DataDna;

/// Finestra di freschezza di default: 24 ore in microsecondi.
pub const DEFAULT_FRESHNESS_WINDOW_US: i64 = 24 * 3600 * 1_000_000;

/// Risultato del controllo di freschezza.
#[derive(Debug, Clone)]
pub struct FreshnessResult {
    /// Età del dato in secondi
    pub age_secs: f64,
    /// Score di freschezza (1.0 = appena creato, 0.0 = scaduto)
    pub freshness_score: f64,
    /// Il dato è dentro la finestra di freschezza
    pub is_fresh: bool,
}

/// Stadio 5 (parziale): Controlla la freschezza di un dato.
///
/// Il punteggio decresce esponenzialmente con l'età:
/// score = exp(-age_secs / half_life_secs)
pub fn check_freshness(ddna: &DataDna, window_us: i64) -> FreshnessResult {
    let now_us = Utc::now().timestamp_micros();
    let age_us = now_us - ddna.temporal.timestamp_us;
    let age_secs = age_us as f64 / 1_000_000.0;

    // Half-life: metà della finestra di freschezza
    let half_life_secs = (window_us as f64 / 1_000_000.0) / 2.0;
    let freshness_score = (-age_secs / half_life_secs).exp().clamp(0.0, 1.0);

    let is_fresh = age_us <= window_us;

    FreshnessResult {
        age_secs,
        freshness_score,
        is_fresh,
    }
}

/// Confronta la freschezza di due versioni dello stesso dato.
/// Restituisce true se `newer` è effettivamente più recente.
pub fn is_newer(newer: &DataDna, older: &DataDna) -> bool {
    newer.temporal.timestamp_us > older.temporal.timestamp_us
}

#[cfg(test)]
mod tests {
    use super::*;
    use varcavia_ddna::identity::KeyPair;

    #[test]
    fn test_fresh_data() {
        let kp = KeyPair::generate();
        let ddna = DataDna::create(b"fresh data", &kp).unwrap();
        let result = check_freshness(&ddna, DEFAULT_FRESHNESS_WINDOW_US);
        assert!(result.is_fresh);
        assert!(result.freshness_score > 0.99);
        assert!(result.age_secs < 1.0);
    }

    #[test]
    fn test_freshness_score_decreases() {
        let kp = KeyPair::generate();
        let ddna = DataDna::create(b"data", &kp).unwrap();
        let result_wide = check_freshness(&ddna, DEFAULT_FRESHNESS_WINDOW_US);
        // Narrow window should give a lower freshness for same age
        let result_narrow = check_freshness(&ddna, 1_000_000); // 1 second window
        assert!(result_wide.freshness_score >= result_narrow.freshness_score);
    }

    #[test]
    fn test_is_newer() {
        let kp = KeyPair::generate();
        let ddna1 = DataDna::create(b"first", &kp).unwrap();
        let ddna2 = DataDna::create(b"second", &kp).unwrap();
        assert!(is_newer(&ddna2, &ddna1));
        assert!(!is_newer(&ddna1, &ddna2));
    }

    #[test]
    fn test_freshness_score_bounds() {
        let kp = KeyPair::generate();
        let ddna = DataDna::create(b"data", &kp).unwrap();
        let result = check_freshness(&ddna, DEFAULT_FRESHNESS_WINDOW_US);
        assert!(result.freshness_score >= 0.0);
        assert!(result.freshness_score <= 1.0);
    }
}
