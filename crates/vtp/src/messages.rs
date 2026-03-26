//! Messaggi di rete scambiati tra nodi VARCAVIA e helper TCP.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Limite massimo per un singolo messaggio: 10 MB.
const MAX_MSG_SIZE: usize = 10 * 1024 * 1024;

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
    /// Richiesta di voto per consenso ARC
    VoteRequest {
        data_id: String,
        data: Vec<u8>,
        ddna_bytes: Vec<u8>,
        domain: String,
    },
    /// Risposta con il voto del peer
    VoteResponse {
        data_id: String,
        node_id: String,
        vote: String,
        confidence: f64,
    },
}

/// Invia un messaggio su un TcpStream (length-prefixed JSON).
pub async fn send_msg(stream: &mut TcpStream, msg: &NodeMessage) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(msg).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    })?;
    let len = (bytes.len() as u32).to_be_bytes();
    stream.write_all(&len).await?;
    stream.write_all(&bytes).await?;
    Ok(())
}

/// Riceve un messaggio da un TcpStream (length-prefixed JSON).
pub async fn recv_msg(stream: &mut TcpStream) -> std::io::Result<NodeMessage> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let msg_len = u32::from_be_bytes(len_buf) as usize;

    if msg_len > MAX_MSG_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Messaggio troppo grande: {msg_len} bytes"),
        ));
    }

    let mut msg_buf = vec![0u8; msg_len];
    stream.read_exact(&mut msg_buf).await?;
    serde_json::from_slice(&msg_buf).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    })
}

/// Invia un messaggio a un indirizzo e riceve la risposta.
pub async fn request(addr: &SocketAddr, msg: &NodeMessage) -> std::io::Result<NodeMessage> {
    let mut stream = TcpStream::connect(addr).await?;
    send_msg(&mut stream, msg).await?;
    recv_msg(&mut stream).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = NodeMessage::Ping { node_id: "test".into() };
        let bytes = serde_json::to_vec(&msg).unwrap();
        let _: NodeMessage = serde_json::from_slice(&bytes).unwrap();
    }

    #[test]
    fn test_vote_request_serialization() {
        let msg = NodeMessage::VoteRequest {
            data_id: "abc".into(),
            data: vec![1, 2, 3],
            ddna_bytes: vec![4, 5, 6],
            domain: "climate".into(),
        };
        let bytes = serde_json::to_vec(&msg).unwrap();
        let restored: NodeMessage = serde_json::from_slice(&bytes).unwrap();
        match restored {
            NodeMessage::VoteRequest { data_id, domain, .. } => {
                assert_eq!(data_id, "abc");
                assert_eq!(domain, "climate");
            }
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn test_vote_response_serialization() {
        let msg = NodeMessage::VoteResponse {
            data_id: "abc".into(),
            node_id: "node1".into(),
            vote: "approve".into(),
            confidence: 0.95,
        };
        let bytes = serde_json::to_vec(&msg).unwrap();
        let _: NodeMessage = serde_json::from_slice(&bytes).unwrap();
    }

    #[tokio::test]
    async fn test_send_recv_roundtrip() {
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let msg = NodeMessage::Ping { node_id: "test-node".into() };
        let msg_clone = msg.clone();

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let received = recv_msg(&mut stream).await.unwrap();
            // Echo back
            send_msg(&mut stream, &received).await.unwrap();
        });

        let mut client = TcpStream::connect(addr).await.unwrap();
        send_msg(&mut client, &msg_clone).await.unwrap();
        let response = recv_msg(&mut client).await.unwrap();

        match response {
            NodeMessage::Ping { node_id } => assert_eq!(node_id, "test-node"),
            _ => panic!("wrong type"),
        }

        server.await.unwrap();
    }
}
