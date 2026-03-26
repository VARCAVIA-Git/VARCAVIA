//! Temporal Proof — Timestamp certificato al microsecondo.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use crate::{DdnaError, Result};

/// Sorgente dell'orologio usato per il timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClockSource {
    /// Orologio di sistema (meno preciso)
    System,
    /// Sincronizzato via NTP
    Ntp,
    /// Sincronizzato via GPS (più preciso)
    Gps,
}

/// Prova temporale certificata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalProof {
    /// Microsecondi da Unix epoch
    pub timestamp_us: i64,
    /// Sorgente dell'orologio
    pub source_clock: ClockSource,
    /// Precisione stimata in microsecondi
    pub precision_us: u32,
}

impl TemporalProof {
    /// Crea una prova temporale con il timestamp corrente.
    pub fn now() -> Self {
        let now = Utc::now();
        TemporalProof {
            timestamp_us: now.timestamp_micros(),
            source_clock: ClockSource::System,
            precision_us: 1_000, // 1ms di precisione per orologio di sistema
        }
    }

    /// Verifica che il timestamp sia plausibile.
    /// Non deve essere nel futuro e non più vecchio di 24 ore (configurabile).
    pub fn verify(&self) -> Result<bool> {
        let now_us = Utc::now().timestamp_micros();
        let max_future_us = 60_000_000; // 60 secondi di tolleranza per clock drift
        let max_age_us: i64 = 24 * 3600 * 1_000_000; // 24 ore

        if self.timestamp_us > now_us + max_future_us {
            return Err(DdnaError::InvalidTimestamp(
                "Timestamp nel futuro oltre la tolleranza".into(),
            ));
        }
        if self.timestamp_us < now_us - max_age_us {
            return Err(DdnaError::InvalidTimestamp(
                "Timestamp troppo vecchio (>24h)".into(),
            ));
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now() {
        let tp = TemporalProof::now();
        assert!(tp.timestamp_us > 0);
        assert!(tp.verify().unwrap());
    }

    #[test]
    fn test_future_timestamp_fails() {
        let tp = TemporalProof {
            timestamp_us: Utc::now().timestamp_micros() + 120_000_000, // 2 min nel futuro
            source_clock: ClockSource::System,
            precision_us: 1_000,
        };
        assert!(tp.verify().is_err());
    }
}
