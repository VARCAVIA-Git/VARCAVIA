//! Unified Channel Abstraction (UCA) — Astrazione del mezzo fisico.
//!
//! Espone un canale logico omogeneo indipendentemente dal mezzo sottostante.
//! Per lo sviluppo locale: usa TCP su localhost.

use serde::{Deserialize, Serialize};

/// Tipo di canale fisico.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChannelType {
    /// TCP/IP locale o remoto
    Tcp,
    /// WebSocket
    WebSocket,
    /// Bluetooth Low Energy (futuro)
    Ble,
    /// LoRaWAN per IoT (futuro)
    LoRa,
    /// Sincronizzazione differita offline (futuro)
    DeferredSync,
}

/// Configurazione di un canale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub channel_type: ChannelType,
    pub address: String,
    pub port: u16,
    pub max_bandwidth_bps: u64,
    pub mtu_bytes: usize,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        ChannelConfig {
            channel_type: ChannelType::Tcp,
            address: "127.0.0.1".to_string(),
            port: 7700,
            max_bandwidth_bps: 1_000_000_000, // 1 Gbps per localhost
            mtu_bytes: 65535,
        }
    }
}
