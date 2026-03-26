//! Validazione locale di un dato + dDNA.

use varcavia_ddna::DataDna;

/// Risultato della validazione locale di un singolo nodo.
#[derive(Debug, Clone)]
pub struct LocalVote {
    pub node_id: [u8; 32],
    pub vote: VoteType,
    pub confidence: f64,
    pub checks_passed: Vec<String>,
    pub checks_failed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VoteType {
    Approve,
    Reject,
    Abstain,
}

/// Esegue tutti i controlli di validazione locale su un dato.
pub fn validate_locally(
    data: &[u8],
    ddna: &DataDna,
    node_id: [u8; 32],
) -> LocalVote {
    let mut passed = Vec::new();
    let mut failed = Vec::new();

    // Check 1: Verifica crittografica del dDNA
    match ddna.verify() {
        Ok(true) => passed.push("crypto_verification".into()),
        _ => failed.push("crypto_verification".into()),
    }

    // Check 2: Content fingerprint corrisponde al dato
    match ddna.verify_content(data) {
        Ok(true) => passed.push("content_fingerprint".into()),
        _ => failed.push("content_fingerprint".into()),
    }

    // Check 3: Timestamp plausibile
    match ddna.temporal.verify() {
        Ok(true) => passed.push("temporal_plausibility".into()),
        _ => failed.push("temporal_plausibility".into()),
    }

    // Check 4: Fonte ha reputazione sufficiente
    if ddna.source.reputation_score >= 0.3 {
        passed.push("source_reputation".into());
    } else {
        failed.push("source_reputation".into());
    }

    // Calcola voto
    let total = passed.len() + failed.len();
    let pass_ratio = passed.len() as f64 / total as f64;

    let vote = if failed.is_empty() {
        VoteType::Approve
    } else if pass_ratio < 0.5 {
        VoteType::Reject
    } else {
        VoteType::Abstain
    };

    LocalVote {
        node_id,
        vote,
        confidence: pass_ratio,
        checks_passed: passed,
        checks_failed: failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use varcavia_ddna::identity::KeyPair;

    #[test]
    fn test_valid_data_approved() {
        let kp = KeyPair::generate();
        let data = b"valid test data";
        let ddna = DataDna::create(data, &kp).unwrap();
        let vote = validate_locally(data, &ddna, [1u8; 32]);
        assert_eq!(vote.vote, VoteType::Approve);
        assert!(vote.checks_failed.is_empty());
    }

    #[test]
    fn test_tampered_data_rejected() {
        let kp = KeyPair::generate();
        let data = b"original data";
        let ddna = DataDna::create(data, &kp).unwrap();
        let vote = validate_locally(b"tampered data", &ddna, [1u8; 32]);
        assert!(vote.checks_failed.contains(&"content_fingerprint".to_string()));
    }
}
