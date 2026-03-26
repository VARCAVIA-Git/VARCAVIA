//! # Universal Access Gateway (UAG)
//!
//! API server REST/GraphQL per esporre i dati VARCAVIA al mondo esterno.
//! Include il Traduttore Universale dei Formati.

#![deny(clippy::all)]

pub mod state;
pub mod server;
pub mod rest;
pub mod graphql;
pub mod translator;
pub mod middleware;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum UagError {
    #[error("Formato non supportato: {0}")]
    UnsupportedFormat(String),
    #[error("Traduzione fallita: {0}")]
    TranslationFailed(String),
    #[error("Autenticazione richiesta")]
    AuthRequired,
    #[error("Rate limit superato")]
    RateLimitExceeded,
}

pub type Result<T> = std::result::Result<T, UagError>;
