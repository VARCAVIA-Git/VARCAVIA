//! Scoring — Punteggio composito di affidabilità per un dato nel CDE.

use serde::{Deserialize, Serialize};

/// Punteggio composito CDE per un dato processato.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdeScore {
    /// Punteggio complessivo (0.0 - 1.0)
    pub overall: f64,
    /// Componente: reputazione della fonte (peso 0.30)
    pub source_reputation: f64,
    /// Componente: coerenza (peso 0.25)
    pub coherence: f64,
    /// Componente: freschezza temporale (peso 0.25)
    pub freshness: f64,
    /// Componente: numero di validazioni indipendenti (peso 0.20)
    pub validations: f64,
}

impl CdeScore {
    /// Calcola il punteggio composito pesato.
    ///
    /// Formula da CLAUDE.md:
    /// score = 0.3 * rep_fonte + 0.25 * coerenza + 0.25 * freschezza + 0.2 * validazioni
    pub fn compute(
        source_reputation: f64,
        coherence: f64,
        freshness: f64,
        num_validations: u32,
    ) -> Self {
        // Normalizza il conteggio validazioni con sigmoid-like
        let validation_score = 1.0 - (-0.5 * num_validations as f64).exp();

        let overall = 0.30 * source_reputation
            + 0.25 * coherence
            + 0.25 * freshness
            + 0.20 * validation_score;

        CdeScore {
            overall: overall.clamp(0.0, 1.0),
            source_reputation,
            coherence,
            freshness,
            validations: validation_score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_score() {
        let score = CdeScore::compute(1.0, 1.0, 1.0, 100);
        assert!(score.overall > 0.9);
    }

    #[test]
    fn test_zero_score() {
        let score = CdeScore::compute(0.0, 0.0, 0.0, 0);
        assert!(score.overall < 0.1);
    }

    #[test]
    fn test_score_bounds() {
        let score = CdeScore::compute(1.0, 1.0, 1.0, 1000);
        assert!(score.overall <= 1.0);
        assert!(score.overall >= 0.0);
    }

    #[test]
    fn test_weights_sum() {
        // With all components at 1.0 and max validations, should approach 1.0
        let score = CdeScore::compute(1.0, 1.0, 1.0, 100);
        assert!(score.overall > 0.95);
    }

    #[test]
    fn test_validation_sigmoid() {
        let score_0 = CdeScore::compute(0.5, 0.5, 0.5, 0);
        let score_1 = CdeScore::compute(0.5, 0.5, 0.5, 1);
        let score_10 = CdeScore::compute(0.5, 0.5, 0.5, 10);
        assert!(score_0.overall < score_1.overall);
        assert!(score_1.overall < score_10.overall);
    }

    #[test]
    fn test_source_reputation_weight() {
        let high_rep = CdeScore::compute(1.0, 0.5, 0.5, 5);
        let low_rep = CdeScore::compute(0.0, 0.5, 0.5, 5);
        assert!(high_rep.overall > low_rep.overall);
        let diff = high_rep.overall - low_rep.overall;
        assert!((diff - 0.30).abs() < 0.01); // Weight is 0.30
    }
}
