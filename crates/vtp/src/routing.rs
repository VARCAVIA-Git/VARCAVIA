//! Gradient Flow Routing (GFR) — Routing ispirato alla dinamica dei fluidi.
//!
//! Ogni nodo mantiene una mappa di potenziale locale aggiornata ogni 50ms.
//! I dati fluiscono seguendo il gradiente di minore resistenza.
//!
//! TODO (Fase 2): implementare il campo di potenziale completo.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metrica di un collegamento verso un nodo vicino.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkMetrics {
    /// ID del nodo vicino
    pub peer_id: [u8; 32],
    /// Latenza media in microsecondi
    pub latency_us: u64,
    /// Banda disponibile in bytes/sec
    pub bandwidth_bps: u64,
    /// Affidabilità storica (0.0 - 1.0)
    pub reliability: f32,
    /// Carico corrente (0.0 = idle, 1.0 = saturo)
    pub load: f32,
}

impl LinkMetrics {
    /// Calcola il "potenziale" del collegamento (più basso = migliore).
    /// Formula: potenziale = latenza * (1 + carico) / (affidabilità * banda)
    pub fn potential(&self) -> f64 {
        let lat = self.latency_us as f64;
        let load_factor = 1.0 + self.load as f64;
        let reliability = self.reliability.max(0.01) as f64;
        let bandwidth = self.bandwidth_bps.max(1) as f64;
        (lat * load_factor) / (reliability * bandwidth)
    }
}

/// Tabella di routing locale basata su Gradient Flow.
#[derive(Debug, Default)]
pub struct RoutingTable {
    /// Metriche per ogni nodo vicino diretto
    pub neighbors: HashMap<[u8; 32], LinkMetrics>,
    /// Ultimo aggiornamento (timestamp microsecondi)
    pub last_update_us: i64,
}

impl RoutingTable {
    /// Seleziona il miglior next-hop per raggiungere una destinazione.
    /// Per ora: sceglie il vicino con potenziale più basso.
    /// TODO: implementare routing multi-hop con propagazione potenziale.
    pub fn best_next_hop(&self, _destination: &[u8; 32]) -> Option<[u8; 32]> {
        self.neighbors
            .iter()
            .min_by(|a, b| a.1.potential().partial_cmp(&b.1.potential()).unwrap())
            .map(|(id, _)| *id)
    }

    /// Aggiorna le metriche di un vicino.
    pub fn update_neighbor(&mut self, metrics: LinkMetrics) {
        self.neighbors.insert(metrics.peer_id, metrics);
        self.last_update_us = chrono::Utc::now().timestamp_micros();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_potential() {
        let good = LinkMetrics {
            peer_id: [1u8; 32],
            latency_us: 1000,
            bandwidth_bps: 1_000_000,
            reliability: 0.99,
            load: 0.1,
        };
        let bad = LinkMetrics {
            peer_id: [2u8; 32],
            latency_us: 50_000,
            bandwidth_bps: 100_000,
            reliability: 0.5,
            load: 0.8,
        };
        assert!(good.potential() < bad.potential());
    }

    #[test]
    fn test_best_next_hop() {
        let mut rt = RoutingTable::default();
        rt.update_neighbor(LinkMetrics {
            peer_id: [1u8; 32], latency_us: 1000, bandwidth_bps: 1_000_000,
            reliability: 0.99, load: 0.1,
        });
        rt.update_neighbor(LinkMetrics {
            peer_id: [2u8; 32], latency_us: 50_000, bandwidth_bps: 100_000,
            reliability: 0.5, load: 0.8,
        });
        let best = rt.best_next_hop(&[3u8; 32]).unwrap();
        assert_eq!(best, [1u8; 32]);
    }
}
