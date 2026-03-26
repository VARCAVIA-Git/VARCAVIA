//! Punteggio di affidabilità composito per un dato.

use serde::{Deserialize, Serialize};

/// Punteggio di affidabilità composito.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityScore {
    /// Punteggio complessivo (0.0 - 1.0)
    pub overall: f64,
    /// Componente: reputazione della fonte
    pub source_reputation: f64,
    /// Componente: coerenza semantica
    pub coherence: f64,
    /// Componente: freschezza temporale
    pub freshness: f64,
    /// Componente: numero di validazioni indipendenti
    pub validation_count: f64,
}

impl ReliabilityScore {
    /// Calcola il punteggio composito pesato.
    pub fn compute(
        source_reputation: f64,
        coherence: f64,
        freshness: f64,
        num_validations: u32,
    ) -> Self {
        // Normalizza il conteggio validazioni (sigmoid-like)
        let validation_score = 1.0 - (-0.5 * num_validations as f64).exp();

        let overall = 0.30 * source_reputation
            + 0.25 * coherence
            + 0.25 * freshness
            + 0.20 * validation_score;

        ReliabilityScore {
            overall: overall.clamp(0.0, 1.0),
            source_reputation,
            coherence,
            freshness,
            validation_count: validation_score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_score() {
        let score = ReliabilityScore::compute(1.0, 1.0, 1.0, 100);
        assert!(score.overall > 0.9);
    }

    #[test]
    fn test_zero_score() {
        let score = ReliabilityScore::compute(0.0, 0.0, 0.0, 0);
        assert!(score.overall < 0.1);
    }
}
