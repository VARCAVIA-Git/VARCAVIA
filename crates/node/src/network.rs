//! Gestione connessioni di rete via TCP puro.
//!
//! Per la Fase 1: TCP puro con serde_json per la comunicazione tra nodi.
//! TODO: migrare a libp2p nella Fase 2 per discovery, noise encryption, yamux multiplexing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

/// Messaggio scambiato tra nodi VARCAVIA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeMessage {
    /// Ping per verificare connettività
    Ping { node_id: String },
    /// Risposta al ping
    Pong { node_id: String },
    /// Annuncio di un nuovo dato con il suo dDNA
    DataAnnounce {
        data_id: String,
        ddna_bytes: Vec<u8>,
    },
    /// Richiesta di un dato specifico
    DataRequest { data_id: String },
    /// Risposta con il dato richiesto
    DataResponse {
        data_id: String,
        data: Option<Vec<u8>>,
    },
    /// Stato del nodo
    StatusRequest,
    /// Risposta con lo stato
    StatusResponse {
        node_id: String,
        data_count: u64,
        uptime_secs: u64,
    },
}

/// Informazioni su un peer connesso.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub node_id: String,
    pub address: SocketAddr,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// Gestore della rete P2P semplificato (TCP puro).
pub struct NetworkManager {
    /// ID di questo nodo
    node_id: String,
    /// Indirizzo di ascolto
    listen_addr: SocketAddr,
    /// Peer connessi
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
}

impl NetworkManager {
    /// Crea un nuovo NetworkManager.
    pub fn new(node_id: String, listen_addr: SocketAddr) -> Self {
        NetworkManager {
            node_id,
            listen_addr,
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Avvia il listener TCP in background.
    pub async fn start_listener(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.listen_addr).await?;
        tracing::info!("Network listener avviato su {}", self.listen_addr);

        let node_id = self.node_id.clone();
        let peers = self.peers.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        tracing::debug!("Nuova connessione da {}", addr);
                        let node_id = node_id.clone();
                        let peers = peers.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, addr, &node_id, &peers).await {
                                tracing::warn!("Errore gestione connessione {}: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Errore accept: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Invia un messaggio a un peer specifico.
    pub async fn send_message(
        &self,
        addr: &SocketAddr,
        message: &NodeMessage,
    ) -> anyhow::Result<NodeMessage> {
        let mut stream = TcpStream::connect(addr).await?;
        let msg_bytes = serde_json::to_vec(message)?;
        let len = (msg_bytes.len() as u32).to_be_bytes();
        stream.write_all(&len).await?;
        stream.write_all(&msg_bytes).await?;

        // Leggi risposta
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let resp_len = u32::from_be_bytes(len_buf) as usize;
        let mut resp_buf = vec![0u8; resp_len];
        stream.read_exact(&mut resp_buf).await?;
        let response: NodeMessage = serde_json::from_slice(&resp_buf)?;
        Ok(response)
    }

    /// Ping a un peer.
    pub async fn ping(&self, addr: &SocketAddr) -> anyhow::Result<bool> {
        let msg = NodeMessage::Ping {
            node_id: self.node_id.clone(),
        };
        match self.send_message(addr, &msg).await {
            Ok(NodeMessage::Pong { .. }) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Restituisce la lista dei peer connessi.
    pub async fn peers(&self) -> Vec<PeerInfo> {
        self.peers.read().await.values().cloned().collect()
    }

    /// Indirizzo di ascolto.
    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }
}

/// Gestisce una singola connessione TCP.
async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    node_id: &str,
    peers: &RwLock<HashMap<String, PeerInfo>>,
) -> anyhow::Result<()> {
    // Leggi messaggio (length-prefixed)
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let msg_len = u32::from_be_bytes(len_buf) as usize;

    if msg_len > 10 * 1024 * 1024 {
        // Max 10MB
        anyhow::bail!("Messaggio troppo grande: {msg_len} bytes");
    }

    let mut msg_buf = vec![0u8; msg_len];
    stream.read_exact(&mut msg_buf).await?;
    let message: NodeMessage = serde_json::from_slice(&msg_buf)?;

    // Processa e genera risposta
    let response = match message {
        NodeMessage::Ping { node_id: peer_id } => {
            peers.write().await.insert(
                peer_id.clone(),
                PeerInfo {
                    node_id: peer_id,
                    address: addr,
                    connected_at: chrono::Utc::now(),
                },
            );
            NodeMessage::Pong {
                node_id: node_id.to_string(),
            }
        }
        NodeMessage::StatusRequest => NodeMessage::StatusResponse {
            node_id: node_id.to_string(),
            data_count: 0,
            uptime_secs: 0,
        },
        other => {
            tracing::debug!("Messaggio non gestito: {:?}", other);
            NodeMessage::Pong {
                node_id: node_id.to_string(),
            }
        }
    };

    // Invia risposta
    let resp_bytes = serde_json::to_vec(&response)?;
    let len = (resp_bytes.len() as u32).to_be_bytes();
    stream.write_all(&len).await?;
    stream.write_all(&resp_bytes).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = NodeMessage::Ping {
            node_id: "test-node".into(),
        };
        let bytes = serde_json::to_vec(&msg).unwrap();
        let restored: NodeMessage = serde_json::from_slice(&bytes).unwrap();
        match restored {
            NodeMessage::Ping { node_id } => assert_eq!(node_id, "test-node"),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_data_announce_serialization() {
        let msg = NodeMessage::DataAnnounce {
            data_id: "abc123".into(),
            ddna_bytes: vec![1, 2, 3, 4],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let restored: NodeMessage = serde_json::from_str(&json).unwrap();
        match restored {
            NodeMessage::DataAnnounce { data_id, ddna_bytes } => {
                assert_eq!(data_id, "abc123");
                assert_eq!(ddna_bytes, vec![1, 2, 3, 4]);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_all_message_variants() {
        let messages = vec![
            NodeMessage::Ping { node_id: "n1".into() },
            NodeMessage::Pong { node_id: "n1".into() },
            NodeMessage::DataAnnounce { data_id: "d1".into(), ddna_bytes: vec![] },
            NodeMessage::DataRequest { data_id: "d1".into() },
            NodeMessage::DataResponse { data_id: "d1".into(), data: None },
            NodeMessage::StatusRequest,
            NodeMessage::StatusResponse { node_id: "n1".into(), data_count: 0, uptime_secs: 0 },
        ];
        for msg in &messages {
            let bytes = serde_json::to_vec(msg).unwrap();
            let _: NodeMessage = serde_json::from_slice(&bytes).unwrap();
        }
    }

    #[tokio::test]
    async fn test_network_manager_creation() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let nm = NetworkManager::new("test-node".into(), addr);
        assert_eq!(nm.listen_addr(), addr);
        assert!(nm.peers().await.is_empty());
    }

    #[tokio::test]
    async fn test_ping_pong() {
        // Avvia un listener
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Gestisci una connessione in background
        tokio::spawn(async move {
            let peers = RwLock::new(HashMap::new());
            if let Ok((stream, client_addr)) = listener.accept().await {
                let _ = handle_connection(stream, client_addr, "server-node", &peers).await;
            }
        });

        // Manda un ping
        let nm = NetworkManager::new("client-node".into(), "127.0.0.1:0".parse().unwrap());
        let result = nm.ping(&addr).await.unwrap();
        assert!(result);
    }
}
