//! # Adaptive Resonance Consensus (ARC)
//!
//! Protocollo di consenso ultra-rapido per VARCAVIA.
//! Raggiunge la finalità in <200ms tramite comitati dinamici
//! selezionati per competenza di dominio e reputazione.
//!
//! ## Differenze rispetto a blockchain
//! - Nessun mining/staking
//! - Consenso locale-e-propagato (non globale)
//! - Comitati scelti per competenza nel dominio del dato
//! - Scalabilità lineare con il numero di nodi

#![deny(clippy::all)]

pub mod committee;
pub mod validation;
pub mod resonance;
pub mod reputation;
pub mod scoring;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArcError {
    #[error("Nessun comitato disponibile per il dominio: {0}")]
    NoCommitteeAvailable(String),

    #[error("Consenso non raggiunto: score {score:.2} < soglia {threshold:.2}")]
    ConsensusNotReached { score: f64, threshold: f64 },

    #[error("Timeout nella validazione: {0}")]
    ValidationTimeout(String),

    #[error("Nodo non autorizzato: {0}")]
    UnauthorizedNode(String),

    #[error("Errore dDNA: {0}")]
    DdnaError(#[from] varcavia_ddna::DdnaError),
}

pub type Result<T> = std::result::Result<T, ArcError>;

/// Risultato di una validazione ARC.
#[derive(Debug, Clone)]
pub enum ValidationResult {
    /// Dato confermato con score di affidabilità
    Confirmed { score: f64 },
    /// Dato rifiutato con motivazione
    Rejected { reason: String },
    /// Dato incerto — richiede escalation a comitato più grande
    Uncertain { score: f64 },
}
