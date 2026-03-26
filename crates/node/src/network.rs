//! Gestione connessioni di rete via TCP puro.
//!
//! Usa i messaggi definiti in varcavia_vtp::messages.
//! Gestisce VoteRequest validando con ARC e replicando il dato.
//! TODO: migrare a libp2p per discovery, noise encryption, yamux multiplexing.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use varcavia_uag::state::{AppState, PREFIX_DATA, PREFIX_DDNA, PREFIX_INFO};
use varcavia_uag::rest::DataInfo;
use varcavia_vtp::messages::{self, NodeMessage};

/// Informazioni su un peer connesso.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub node_id: String,
    pub address: SocketAddr,
}

/// Gestore della rete P2P semplificato (TCP puro).
pub struct NetworkManager {
    /// ID di questo nodo
    node_id: String,
    /// Indirizzo di ascolto
    listen_addr: SocketAddr,
    /// Peer connessi (node_id → info)
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    /// Stato condiviso per accesso allo storage
    state: Arc<AppState>,
}

impl NetworkManager {
    /// Crea un nuovo NetworkManager.
    pub fn new(node_id: String, listen_addr: SocketAddr, state: Arc<AppState>) -> Self {
        NetworkManager {
            node_id,
            listen_addr,
            peers: Arc::new(RwLock::new(HashMap::new())),
            state,
        }
    }

    /// Avvia il listener TCP in background.
    pub async fn start_listener(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.listen_addr).await?;
        tracing::info!("Network listener avviato su {}", self.listen_addr);

        let node_id = self.node_id.clone();
        let peers = self.peers.clone();
        let state = self.state.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((mut stream, addr)) => {
                        let node_id = node_id.clone();
                        let peers = peers.clone();
                        let state = state.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(
                                &mut stream, addr, &node_id, &peers, &state,
                            ).await {
                                tracing::warn!("Errore connessione {}: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => tracing::error!("Errore accept: {}", e),
                }
            }
        });

        Ok(())
    }

    /// Pinga un peer e lo registra.
    pub async fn ping_peer(&self, addr: &SocketAddr) -> anyhow::Result<String> {
        let msg = NodeMessage::Ping {
            node_id: self.node_id.clone(),
        };
        let response = messages::request(addr, &msg).await?;
        if let NodeMessage::Pong { node_id } = response {
            self.peers.write().await.insert(
                node_id.clone(),
                PeerInfo {
                    node_id: node_id.clone(),
                    address: *addr,
                },
            );
            Ok(node_id)
        } else {
            anyhow::bail!("Risposta inattesa al ping")
        }
    }

    /// Connetti a tutti i bootstrap peers.
    pub async fn connect_to_peers(&self, addrs: &[SocketAddr]) {
        for addr in addrs {
            match self.ping_peer(addr).await {
                Ok(peer_id) => {
                    tracing::info!("Connesso a peer {} ({})", &peer_id[..16], addr);
                    self.state.add_peer(*addr).await;
                }
                Err(e) => {
                    tracing::warn!("Impossibile connettersi a {}: {}", addr, e);
                }
            }
        }
    }

    /// Indirizzo di ascolto.
    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }
}

/// Gestisce una singola connessione TCP in ingresso.
async fn handle_connection(
    stream: &mut tokio::net::TcpStream,
    addr: SocketAddr,
    node_id: &str,
    peers: &RwLock<HashMap<String, PeerInfo>>,
    state: &AppState,
) -> anyhow::Result<()> {
    let message = messages::recv_msg(stream).await?;

    let response = match message {
        NodeMessage::Ping { node_id: peer_id } => {
            peers.write().await.insert(
                peer_id.clone(),
                PeerInfo {
                    node_id: peer_id,
                    address: addr,
                },
            );
            NodeMessage::Pong {
                node_id: node_id.to_string(),
            }
        }

        NodeMessage::StatusRequest => NodeMessage::StatusResponse {
            node_id: node_id.to_string(),
            data_count: state.data_count(),
            uptime_secs: state.uptime_secs(),
        },

        NodeMessage::VoteRequest {
            data_id,
            data,
            ddna_bytes,
            domain,
        } => {
            handle_vote_request(node_id, state, &data_id, &data, &ddna_bytes, &domain)
        }

        NodeMessage::DataRequest { data_id } => {
            let data_key = AppState::make_key(PREFIX_DATA, &data_id);
            let data = state.db.get(data_key).ok().flatten().map(|v| v.to_vec());
            NodeMessage::DataResponse { data_id, data }
        }

        other => {
            tracing::debug!("Messaggio non gestito: {:?}", other);
            NodeMessage::Pong {
                node_id: node_id.to_string(),
            }
        }
    };

    messages::send_msg(stream, &response).await?;
    Ok(())
}

/// Gestisce una richiesta di voto: valida il dato con ARC e lo replica localmente.
fn handle_vote_request(
    node_id: &str,
    state: &AppState,
    data_id: &str,
    data: &[u8],
    ddna_bytes: &[u8],
    domain: &str,
) -> NodeMessage {
    // 1. Deserializza il dDNA
    let ddna = match varcavia_ddna::DataDna::from_bytes(ddna_bytes) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("VoteRequest: dDNA invalido per {}: {}", data_id, e);
            return NodeMessage::VoteResponse {
                data_id: data_id.to_string(),
                node_id: node_id.to_string(),
                vote: "reject".into(),
                confidence: 0.0,
            };
        }
    };

    // 2. Valida localmente con ARC
    let mut validator_id = [0u8; 32];
    if let Ok(bytes) = hex::decode(node_id) {
        let len = bytes.len().min(32);
        validator_id[..len].copy_from_slice(&bytes[..len]);
    }
    let vote = varcavia_arc::validation::validate_locally(data, &ddna, validator_id);

    let vote_str = match vote.vote {
        varcavia_arc::validation::VoteType::Approve => "approve",
        varcavia_arc::validation::VoteType::Reject => "reject",
        varcavia_arc::validation::VoteType::Abstain => "abstain",
    };

    tracing::info!(
        "Voto per {}: {} (confidence: {:.2})",
        data_id,
        vote_str,
        vote.confidence
    );

    // 3. Se approvato, replica il dato localmente
    if vote_str == "approve" {
        let data_key = AppState::make_key(PREFIX_DATA, data_id);
        let ddna_key = AppState::make_key(PREFIX_DDNA, data_id);
        let info_key = AppState::make_key(PREFIX_INFO, data_id);

        let _ = state.db.insert(data_key, data);
        let _ = state.db.insert(ddna_key, ddna_bytes);

        let info = DataInfo {
            domain: domain.to_string(),
            score: vote.confidence,
            inserted_at_us: chrono::Utc::now().timestamp_micros(),
        };
        if let Ok(info_json) = serde_json::to_vec(&info) {
            let _ = state.db.insert(info_key, info_json);
        }

        tracing::info!("Dato {} replicato localmente", data_id);
    }

    NodeMessage::VoteResponse {
        data_id: data_id.to_string(),
        node_id: node_id.to_string(),
        vote: vote_str.into(),
        confidence: vote.confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use varcavia_cde::pipeline::PipelineConfig;
    use varcavia_ddna::identity::KeyPair;

    fn temp_state() -> Arc<AppState> {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let kp = KeyPair::generate();
        Arc::new(AppState::new(db, kp.secret_bytes(), PipelineConfig::default()))
    }

    #[test]
    fn test_vote_request_approve() {
        let state = temp_state();
        let kp = KeyPair::generate();
        let data = b"valid test data for voting";
        let ddna = varcavia_ddna::DataDna::create(data, &kp).unwrap();
        let ddna_bytes = ddna.to_bytes().unwrap();

        let response = handle_vote_request(
            &state.node_id,
            &state,
            "test-data-id",
            data,
            &ddna_bytes,
            "test",
        );

        match response {
            NodeMessage::VoteResponse { vote, confidence, .. } => {
                assert_eq!(vote, "approve");
                assert!(confidence > 0.9);
            }
            _ => panic!("Expected VoteResponse"),
        }

        // Verifica che il dato è stato replicato
        let data_key = AppState::make_key(PREFIX_DATA, "test-data-id");
        assert!(state.db.get(data_key).unwrap().is_some());
    }

    #[test]
    fn test_vote_request_reject_bad_ddna() {
        let state = temp_state();
        let response = handle_vote_request(
            &state.node_id,
            &state,
            "bad-id",
            b"data",
            b"invalid ddna bytes",
            "test",
        );

        match response {
            NodeMessage::VoteResponse { vote, .. } => {
                assert_eq!(vote, "reject");
            }
            _ => panic!("Expected VoteResponse"),
        }
    }

    #[test]
    fn test_vote_request_reject_tampered() {
        let state = temp_state();
        let kp = KeyPair::generate();
        let data = b"original data";
        let ddna = varcavia_ddna::DataDna::create(data, &kp).unwrap();
        let ddna_bytes = ddna.to_bytes().unwrap();

        // Dato diverso da quello firmato
        let response = handle_vote_request(
            &state.node_id,
            &state,
            "tampered-id",
            b"tampered data",
            &ddna_bytes,
            "test",
        );

        match response {
            NodeMessage::VoteResponse { vote, .. } => {
                assert_ne!(vote, "approve");
            }
            _ => panic!("Expected VoteResponse"),
        }
    }

    #[tokio::test]
    async fn test_ping_and_vote_over_network() {
        let state = temp_state();
        let nm = NetworkManager::new(
            state.node_id.clone(),
            "127.0.0.1:0".parse().unwrap(),
            state.clone(),
        );
        nm.start_listener().await.unwrap();

        // Il listener si bind su porta random, usiamo un approccio diverso
        // per il test: creiamo un listener manuale
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let state_clone = state.clone();

        tokio::spawn(async move {
            let (mut stream, peer_addr) = listener.accept().await.unwrap();
            let peers: RwLock<HashMap<String, PeerInfo>> = RwLock::new(HashMap::new());
            let _ = handle_connection(
                &mut stream, peer_addr, &state_clone.node_id, &peers, &state_clone,
            ).await;
        });

        // Ping
        let response = messages::request(
            &addr,
            &NodeMessage::Ping { node_id: "client".into() },
        ).await.unwrap();
        assert!(matches!(response, NodeMessage::Pong { .. }));
    }

    #[tokio::test]
    async fn test_vote_over_network() {
        let state = temp_state();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let state_clone = state.clone();

        tokio::spawn(async move {
            let (mut stream, peer_addr) = listener.accept().await.unwrap();
            let peers: RwLock<HashMap<String, PeerInfo>> = RwLock::new(HashMap::new());
            let _ = handle_connection(
                &mut stream, peer_addr, &state_clone.node_id, &peers, &state_clone,
            ).await;
        });

        // Create valid data+dDNA and request a vote
        let kp = KeyPair::generate();
        let data = b"network vote test data";
        let ddna = varcavia_ddna::DataDna::create(data, &kp).unwrap();
        let ddna_bytes = ddna.to_bytes().unwrap();

        let response = messages::request(
            &addr,
            &NodeMessage::VoteRequest {
                data_id: "net-test".into(),
                data: data.to_vec(),
                ddna_bytes,
                domain: "test".into(),
            },
        ).await.unwrap();

        match response {
            NodeMessage::VoteResponse { vote, confidence, .. } => {
                assert_eq!(vote, "approve");
                assert!(confidence > 0.5);
            }
            _ => panic!("Expected VoteResponse"),
        }

        // Verify the peer replicated the data
        let data_key = AppState::make_key(PREFIX_DATA, "net-test");
        assert!(state.db.get(data_key).unwrap().is_some());
    }
}
