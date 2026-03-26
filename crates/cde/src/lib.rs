//! # Clean Data Engine (CDE)
//!
//! Pipeline di purificazione automatica dei dati in 6 stadi:
//! 1. Deduplicazione hash esatto
//! 2. Deduplicazione near-duplicate (LSH)
//! 3. Deduplicazione semantica (AI embedding)
//! 4. Validazione fonte
//! 5. Normalizzazione in VUF
//! 6. Scoring affidabilità composito

#![deny(clippy::all)]

pub mod pipeline;
pub mod dedup;
pub mod validation;
pub mod freshness;
pub mod normalize;
pub mod scoring;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CdeError {
    #[error("Dato duplicato: {0}")]
    DuplicateData(String),
    #[error("Validazione fonte fallita: {0}")]
    SourceValidationFailed(String),
    #[error("Normalizzazione fallita: {0}")]
    NormalizationFailed(String),
    #[error("Errore AI agent: {0}")]
    AiAgentError(String),
}

pub type Result<T> = std::result::Result<T, CdeError>;
