//! Consenso distribuito — coordina la validazione ARC con i peer.
//!
//! Quando un dato viene inserito, il nodo proponente:
//! 1. Invia VoteRequest a tutti i peer
//! 2. Raccoglie VoteResponse con timeout
//! 3. Aggrega i voti con ARC resonance
//! 4. Aggiorna lo score se il consenso è raggiunto

use std::sync::Arc;
use varcavia_arc::resonance;
use varcavia_arc::validation::{LocalVote, VoteType};
use varcavia_vtp::messages::{self, NodeMessage};

use crate::state::{AppState, PREFIX_INFO};
use crate::rest::DataInfo;

/// Risultato del processo di consenso.
#[derive(Debug)]
pub struct ConsensusResult {
    pub data_id: String,
    pub score: f64,
    pub votes_received: usize,
    pub confirmed: bool,
}

/// Timeout per la raccolta voti (ms).
const VOTE_TIMEOUT_MS: u64 = 5000;

/// Esegue il consenso distribuito per un dato appena inserito.
///
/// Invia VoteRequest a tutti i peer e raccoglie i voti.
/// Aggiorna lo score nel database se il consenso è raggiunto.
pub async fn run_consensus(
    state: &Arc<AppState>,
    data_id: &str,
    data: &[u8],
    ddna_bytes: &[u8],
    domain: &str,
) -> ConsensusResult {
    let peers = state.get_peers().await;

    if peers.is_empty() {
        tracing::debug!("Nessun peer per il consenso di {}", data_id);
        return ConsensusResult {
            data_id: data_id.to_string(),
            score: 0.0,
            votes_received: 0,
            confirmed: false,
        };
    }

    tracing::info!(
        "Avvio consenso per {} con {} peer",
        data_id,
        peers.len()
    );

    let vote_request = NodeMessage::VoteRequest {
        data_id: data_id.to_string(),
        data: data.to_vec(),
        ddna_bytes: ddna_bytes.to_vec(),
        domain: domain.to_string(),
    };

    // Invia VoteRequest a tutti i peer in parallelo, con timeout
    let mut vote_handles = Vec::new();
    for peer_addr in &peers {
        let req = vote_request.clone();
        let addr = *peer_addr;
        let handle = tokio::spawn(async move {
            let timeout = tokio::time::Duration::from_millis(VOTE_TIMEOUT_MS);
            match tokio::time::timeout(timeout, messages::request(&addr, &req)).await {
                Ok(Ok(response)) => Some(response),
                Ok(Err(e)) => {
                    tracing::warn!("Errore comunicazione con peer {}: {}", addr, e);
                    None
                }
                Err(_) => {
                    tracing::warn!("Timeout voto da peer {}", addr);
                    None
                }
            }
        });
        vote_handles.push(handle);
    }

    // Raccogli i voti
    let mut local_votes: Vec<LocalVote> = Vec::new();
    for handle in vote_handles {
        if let Ok(Some(NodeMessage::VoteResponse {
            node_id,
            vote,
            confidence,
            ..
        })) = handle.await
        {
            let vote_type = match vote.as_str() {
                "approve" => VoteType::Approve,
                "reject" => VoteType::Reject,
                _ => VoteType::Abstain,
            };
            let mut nid = [0u8; 32];
            if let Ok(bytes) = hex::decode(&node_id) {
                let len = bytes.len().min(32);
                nid[..len].copy_from_slice(&bytes[..len]);
            }
            local_votes.push(LocalVote {
                node_id: nid,
                vote: vote_type,
                confidence,
                checks_passed: vec![],
                checks_failed: vec![],
            });
        }
    }

    let votes_received = local_votes.len();
    let score = resonance::aggregate_votes(&local_votes);
    let outcome = resonance::determine_outcome(score, resonance::DEFAULT_THRESHOLD);
    let confirmed = matches!(outcome, varcavia_arc::ValidationResult::Confirmed { .. });

    tracing::info!(
        "Consenso per {}: score={:.2}, voti={}, confermato={}",
        data_id,
        score,
        votes_received,
        confirmed
    );

    // Se confermato, aggiorna lo score nel database
    if confirmed {
        let info_key = AppState::make_key(PREFIX_INFO, data_id);
        if let Ok(Some(info_bytes)) = state.db.get(&info_key) {
            if let Ok(mut info) = serde_json::from_slice::<DataInfo>(&info_bytes) {
                // Boost score: media tra score CDE e score consenso
                info.score = (info.score + score) / 2.0;
                if let Ok(updated) = serde_json::to_vec(&info) {
                    let _ = state.db.insert(info_key, updated);
                }
            }
        }
    }

    ConsensusResult {
        data_id: data_id.to_string(),
        score,
        votes_received,
        confirmed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use varcavia_cde::pipeline::PipelineConfig;
    use varcavia_ddna::identity::KeyPair;

    fn temp_state() -> Arc<AppState> {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let kp = KeyPair::generate();
        Arc::new(AppState::new(db, kp.secret_bytes(), PipelineConfig::default()))
    }

    #[tokio::test]
    async fn test_consensus_no_peers() {
        let state = temp_state();
        let result = run_consensus(&state, "test-id", b"data", b"ddna", "test").await;
        assert_eq!(result.votes_received, 0);
        assert!(!result.confirmed);
    }

    #[tokio::test]
    async fn test_consensus_unreachable_peer() {
        let state = temp_state();
        // Peer inesistente — timeout
        state.add_peer("127.0.0.1:59999".parse().unwrap()).await;
        let result = run_consensus(&state, "test-id", b"data", b"ddna", "test").await;
        assert_eq!(result.votes_received, 0);
        assert!(!result.confirmed);
    }
}
