//! # VARCAVIA Transport Protocol (VTP)
//!
//! Protocollo di trasporto adattivo multi-percorso per dati strutturati.
//! Sostituisce TCP/IP per le comunicazioni interne alla rete VARCAVIA con:
//! - Semantic Priority Queuing
//! - Gradient Flow Routing (GFR)
//! - Delta Compression
//! - Multi-Channel Bonding
//! - Store-and-Forward con CRDT sync

#![deny(clippy::all)]

pub mod packet;
pub mod priority;
pub mod routing;
pub mod compression;
pub mod channel;
pub mod sync;
pub mod messages;

use thiserror::Error;

/// Errori del modulo VTP
#[derive(Error, Debug)]
pub enum VtpError {
    /// Errore di trasmissione
    #[error("Errore di trasmissione: {0}")]
    TransmissionError(String),

    /// Errore di compressione
    #[error("Errore di compressione: {0}")]
    CompressionError(String),

    /// Timeout nella trasmissione
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Canale non disponibile
    #[error("Canale non disponibile: {0}")]
    ChannelUnavailable(String),

    /// Errore di routing
    #[error("Routing fallito: {0}")]
    RoutingError(String),
}

pub type Result<T> = std::result::Result<T, VtpError>;
