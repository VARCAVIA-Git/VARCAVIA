//! Sistema di reputazione dei nodi.

use serde::{Deserialize, Serialize};

/// Record di reputazione di un nodo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeReputation {
    pub node_id: [u8; 32],
    /// Reputazione corrente (0.0 - 1.0)
    pub score: f64,
    /// Numero totale di validazioni eseguite
    pub total_validations: u64,
    /// Numero di validazioni corrette
    pub correct_validations: u64,
    /// Tasso di decay per periodo (0.01 = 1% per periodo)
    pub decay_rate: f64,
}

impl NodeReputation {
    /// Crea una nuova reputazione per un nodo (default 0.5).
    pub fn new(node_id: [u8; 32]) -> Self {
        NodeReputation {
            node_id,
            score: 0.5,
            total_validations: 0,
            correct_validations: 0,
            decay_rate: 0.01,
        }
    }

    /// Aggiorna la reputazione dopo una validazione.
    pub fn update(&mut self, was_correct: bool) {
        self.total_validations += 1;
        if was_correct {
            self.correct_validations += 1;
            // Incremento decrescente: più reputazione hai, più lento cresce
            self.score += (1.0 - self.score) * 0.05;
        } else {
            // Penalità più forte: perdere reputazione è facile
            self.score -= self.score * 0.15;
        }
        self.score = self.score.clamp(0.0, 1.0);
    }

    /// Applica il decay temporale.
    pub fn apply_decay(&mut self) {
        self.score *= 1.0 - self.decay_rate;
        self.score = self.score.max(0.0);
    }

    /// Percentuale di validazioni corrette.
    pub fn accuracy(&self) -> f64 {
        if self.total_validations == 0 {
            return 0.5;
        }
        self.correct_validations as f64 / self.total_validations as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_growth() {
        let mut rep = NodeReputation::new([1u8; 32]);
        let initial = rep.score;
        rep.update(true);
        assert!(rep.score > initial);
    }

    #[test]
    fn test_reputation_penalty() {
        let mut rep = NodeReputation::new([1u8; 32]);
        rep.score = 0.8;
        rep.update(false);
        assert!(rep.score < 0.8);
    }

    #[test]
    fn test_reputation_bounds() {
        let mut rep = NodeReputation::new([1u8; 32]);
        for _ in 0..1000 { rep.update(true); }
        assert!(rep.score <= 1.0);
        for _ in 0..1000 { rep.update(false); }
        assert!(rep.score >= 0.0);
    }
}
