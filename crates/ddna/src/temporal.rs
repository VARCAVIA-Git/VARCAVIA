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

    /// Crea una prova temporale con un timestamp specifico.
    pub fn with_timestamp(timestamp_us: i64, source: ClockSource, precision_us: u32) -> Self {
        TemporalProof {
            timestamp_us,
            source_clock: source,
            precision_us,
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

    /// Restituisce l'età del timestamp in secondi.
    pub fn age_secs(&self) -> f64 {
        let now_us = Utc::now().timestamp_micros();
        (now_us - self.timestamp_us) as f64 / 1_000_000.0
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

    #[test]
    fn test_old_timestamp_fails() {
        let tp = TemporalProof {
            timestamp_us: Utc::now().timestamp_micros() - 48 * 3600 * 1_000_000, // 48h ago
            source_clock: ClockSource::System,
            precision_us: 1_000,
        };
        assert!(tp.verify().is_err());
    }

    #[test]
    fn test_age_secs() {
        let tp = TemporalProof::now();
        let age = tp.age_secs();
        assert!(age < 1.0); // Should be very recent
    }

    #[test]
    fn test_with_timestamp() {
        let ts = Utc::now().timestamp_micros();
        let tp = TemporalProof::with_timestamp(ts, ClockSource::Ntp, 100);
        assert_eq!(tp.timestamp_us, ts);
        assert_eq!(tp.source_clock, ClockSource::Ntp);
        assert_eq!(tp.precision_us, 100);
    }

    #[test]
    fn test_clock_source_variants() {
        for source in [ClockSource::System, ClockSource::Ntp, ClockSource::Gps] {
            let tp = TemporalProof::with_timestamp(
                Utc::now().timestamp_micros(),
                source.clone(),
                1_000,
            );
            assert!(tp.verify().is_ok());
        }
    }
}
