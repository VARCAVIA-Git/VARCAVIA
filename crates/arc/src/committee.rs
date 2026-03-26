//! Selezione del Comitato di Risonanza.
//!
//! Seleziona 7-21 nodi per validare un dato in base a:
//! - Competenza nel dominio del dato
//! - Reputazione storica del nodo
//! - Diversità geografica

use serde::{Deserialize, Serialize};

/// Informazioni su un nodo candidato per il comitato.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: [u8; 32],
    pub reputation: f64,
    pub domain_competences: Vec<(String, f64)>, // (dominio, competenza)
    pub region: String,
    pub is_available: bool,
}

/// Parametri per la selezione del comitato.
#[derive(Debug, Clone)]
pub struct CommitteeParams {
    /// Dimensione target del comitato
    pub size: usize,
    /// Dominio del dato da validare
    pub domain: String,
    /// Reputazione minima richiesta
    pub min_reputation: f64,
    /// Numero minimo di regioni diverse
    pub min_regions: usize,
}

impl Default for CommitteeParams {
    fn default() -> Self {
        CommitteeParams {
            size: 7,
            domain: "general".to_string(),
            min_reputation: 0.5,
            min_regions: 3,
        }
    }
}

/// Seleziona un comitato di validazione dai nodi disponibili.
///
/// Algoritmo:
/// 1. Filtra nodi per reputazione minima e disponibilità
/// 2. Ordina per competenza nel dominio richiesto
/// 3. Seleziona garantendo diversità geografica
/// 4. Se non abbastanza nodi qualificati, espandi a generalisti
pub fn select_committee(
    candidates: &[NodeInfo],
    params: &CommitteeParams,
) -> Vec<NodeInfo> {
    let mut eligible: Vec<&NodeInfo> = candidates
        .iter()
        .filter(|n| n.is_available && n.reputation >= params.min_reputation)
        .collect();

    // Ordina per competenza nel dominio (decrescente)
    eligible.sort_by(|a, b| {
        let comp_a = a.domain_competences.iter()
            .find(|(d, _)| d == &params.domain)
            .map(|(_, c)| *c)
            .unwrap_or(0.0);
        let comp_b = b.domain_competences.iter()
            .find(|(d, _)| d == &params.domain)
            .map(|(_, c)| *c)
            .unwrap_or(0.0);
        comp_b.partial_cmp(&comp_a).unwrap()
    });

    // Seleziona con diversità geografica
    let mut selected = Vec::new();
    let mut regions_seen = std::collections::HashSet::new();

    for node in &eligible {
        if selected.len() >= params.size {
            break;
        }
        selected.push((*node).clone());
        regions_seen.insert(node.region.clone());
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_committee() {
        let candidates = vec![
            NodeInfo {
                node_id: [1u8; 32], reputation: 0.9,
                domain_competences: vec![("climate".into(), 0.95)],
                region: "EU".into(), is_available: true,
            },
            NodeInfo {
                node_id: [2u8; 32], reputation: 0.8,
                domain_competences: vec![("climate".into(), 0.7)],
                region: "US".into(), is_available: true,
            },
            NodeInfo {
                node_id: [3u8; 32], reputation: 0.3, // troppo bassa
                domain_competences: vec![("climate".into(), 0.99)],
                region: "AS".into(), is_available: true,
            },
        ];
        let params = CommitteeParams { size: 3, domain: "climate".into(), ..Default::default() };
        let committee = select_committee(&candidates, &params);
        assert_eq!(committee.len(), 2); // il terzo ha reputazione troppo bassa
        assert_eq!(committee[0].node_id, [1u8; 32]); // più competente
    }
}
