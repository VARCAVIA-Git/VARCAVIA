//! Propagazione a onda del consenso — Fase di Risonanza.

use super::validation::{LocalVote, VoteType};

/// Soglia di default per conferma (2/3 dei voti pesati).
pub const DEFAULT_THRESHOLD: f64 = 0.67;

/// Aggrega i voti pesati per reputazione dei validatori.
pub fn aggregate_votes(votes: &[LocalVote], reputations: &[[u8; 32]; 0]) -> f64 {
    // Per ora: media semplice della confidence dei voti Approve.
    // TODO: pesare per reputazione del nodo.
    if votes.is_empty() {
        return 0.0;
    }

    let weighted_sum: f64 = votes.iter().map(|v| {
        match v.vote {
            VoteType::Approve => v.confidence,
            VoteType::Reject => 0.0,
            VoteType::Abstain => v.confidence * 0.5,
        }
    }).sum();

    weighted_sum / votes.len() as f64
}

/// Determina il risultato del consenso.
pub fn determine_outcome(
    score: f64,
    threshold: f64,
) -> super::ValidationResult {
    if score >= threshold {
        super::ValidationResult::Confirmed { score }
    } else if score < threshold * 0.5 {
        super::ValidationResult::Rejected {
            reason: format!("Score {score:.2} sotto la soglia critica {:.2}", threshold * 0.5),
        }
    } else {
        super::ValidationResult::Uncertain { score }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{LocalVote, VoteType};

    #[test]
    fn test_all_approve() {
        let votes = vec![
            LocalVote { node_id: [1u8; 32], vote: VoteType::Approve, confidence: 1.0,
                checks_passed: vec![], checks_failed: vec![] },
            LocalVote { node_id: [2u8; 32], vote: VoteType::Approve, confidence: 0.9,
                checks_passed: vec![], checks_failed: vec![] },
        ];
        let score = aggregate_votes(&votes, &[]);
        assert!(score > DEFAULT_THRESHOLD);
    }

    #[test]
    fn test_outcome_confirmed() {
        let result = determine_outcome(0.85, DEFAULT_THRESHOLD);
        assert!(matches!(result, super::super::ValidationResult::Confirmed { .. }));
    }

    #[test]
    fn test_outcome_rejected() {
        let result = determine_outcome(0.1, DEFAULT_THRESHOLD);
        assert!(matches!(result, super::super::ValidationResult::Rejected { .. }));
    }
}
