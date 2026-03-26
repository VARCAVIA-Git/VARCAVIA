//! REST API endpoints per VARCAVIA.
//!
//! Endpoints da CLAUDE.md:
//! - POST   /api/v1/data              — Inserisci un nuovo dato
//! - GET    /api/v1/data/{id}         — Recupera un dato per ID
//! - GET    /api/v1/data/{id}/dna     — Recupera solo il dDNA
//! - POST   /api/v1/data/query        — Query semantica
//! - POST   /api/v1/data/verify       — Verifica autenticità
//! - GET    /api/v1/data/{id}/score   — Punteggio affidabilità
//! - DELETE /api/v1/data/{id}         — Soft delete
//! - GET    /api/v1/node/status       — Stato del nodo
//! - GET    /api/v1/node/peers        — Lista peer
//! - GET    /api/v1/node/stats        — Statistiche
//! - GET    /api/v1/network/health    — Salute rete
//! - GET    /api/v1/network/topology  — Topologia
//! - POST   /api/v1/translate         — Conversione formato

use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    routing::{get, post, delete},
};
use serde::{Deserialize, Serialize};

// === Request/Response types ===

#[derive(Debug, Deserialize)]
pub struct InsertDataRequest {
    pub content: String,
    pub domain: String,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct InsertDataResponse {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct DataResponse {
    pub id: String,
    pub content: String,
    pub domain: String,
    pub score: f64,
}

#[derive(Debug, Serialize)]
pub struct NodeStatusResponse {
    pub node_id: String,
    pub version: String,
    pub status: String,
    pub uptime_secs: u64,
    pub data_count: u64,
}

#[derive(Debug, Serialize)]
pub struct NodeStatsResponse {
    pub total_data: u64,
    pub total_validations: u64,
    pub avg_score: f64,
}

#[derive(Debug, Serialize)]
pub struct PeerResponse {
    pub node_id: String,
    pub address: String,
}

#[derive(Debug, Serialize)]
pub struct NetworkHealthResponse {
    pub status: String,
    pub connected_peers: usize,
    pub network_score: f64,
}

#[derive(Debug, Serialize)]
pub struct ScoreResponse {
    pub id: String,
    pub overall: f64,
    pub source_reputation: f64,
    pub coherence: f64,
    pub freshness: f64,
    pub validations: f64,
}

#[derive(Debug, Deserialize)]
pub struct TranslateRequest {
    pub data: serde_json::Value,
    pub from_format: String,
    pub to_format: String,
}

#[derive(Debug, Serialize)]
pub struct TranslateResponse {
    pub result: String,
    pub format: String,
}

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub query: String,
    pub domain: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub id: String,
    pub content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub id: String,
    pub verified: bool,
    pub details: String,
}

// === Route builders ===

/// Route per /api/v1/data/*
pub fn data_routes() -> Router {
    Router::new()
        .route("/api/v1/data", post(insert_data))
        .route("/api/v1/data/query", post(query_data))
        .route("/api/v1/data/verify", post(verify_data))
        .route("/api/v1/data/{id}", get(get_data))
        .route("/api/v1/data/{id}", delete(delete_data))
        .route("/api/v1/data/{id}/dna", get(get_data_dna))
        .route("/api/v1/data/{id}/score", get(get_data_score))
}

/// Route per /api/v1/node/*
pub fn node_routes() -> Router {
    Router::new()
        .route("/api/v1/node/status", get(node_status))
        .route("/api/v1/node/peers", get(node_peers))
        .route("/api/v1/node/stats", get(node_stats))
}

/// Route per /api/v1/network/*
pub fn network_routes() -> Router {
    Router::new()
        .route("/api/v1/network/health", get(network_health))
        .route("/api/v1/network/topology", get(network_topology))
}

/// Route per /api/v1/translate
pub fn translate_routes() -> Router {
    Router::new().route("/api/v1/translate", post(translate))
}

// === Handlers ===

async fn insert_data(
    Json(req): Json<InsertDataRequest>,
) -> (StatusCode, Json<InsertDataResponse>) {
    let id = hex::encode(blake3::hash(req.content.as_bytes()).as_bytes());
    (
        StatusCode::CREATED,
        Json(InsertDataResponse {
            id,
            status: "accepted".into(),
        }),
    )
}

async fn get_data(Path(id): Path<String>) -> (StatusCode, Json<serde_json::Value>) {
    // TODO: lookup dal storage reale
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": "Dato non trovato",
            "id": id
        })),
    )
}

async fn get_data_dna(Path(id): Path<String>) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": "dDNA non trovato",
            "id": id
        })),
    )
}

async fn get_data_score(Path(id): Path<String>) -> Json<ScoreResponse> {
    // TODO: lookup dal storage reale
    Json(ScoreResponse {
        id,
        overall: 0.0,
        source_reputation: 0.0,
        coherence: 0.0,
        freshness: 0.0,
        validations: 0.0,
    })
}

async fn delete_data(Path(id): Path<String>) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "id": id,
            "status": "marked_obsolete"
        })),
    )
}

async fn query_data(Json(_req): Json<QueryRequest>) -> Json<Vec<DataResponse>> {
    // TODO: implementare query semantica
    Json(vec![])
}

async fn verify_data(Json(req): Json<VerifyRequest>) -> Json<VerifyResponse> {
    // TODO: implementare verifica reale
    Json(VerifyResponse {
        id: req.id,
        verified: false,
        details: "Verifica non ancora implementata".into(),
    })
}

async fn node_status() -> Json<NodeStatusResponse> {
    Json(NodeStatusResponse {
        node_id: "local-dev".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        status: "running".into(),
        uptime_secs: 0,
        data_count: 0,
    })
}

async fn node_peers() -> Json<Vec<PeerResponse>> {
    Json(vec![])
}

async fn node_stats() -> Json<NodeStatsResponse> {
    Json(NodeStatsResponse {
        total_data: 0,
        total_validations: 0,
        avg_score: 0.0,
    })
}

async fn network_health() -> Json<NetworkHealthResponse> {
    Json(NetworkHealthResponse {
        status: "healthy".into(),
        connected_peers: 0,
        network_score: 1.0,
    })
}

async fn network_topology() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "nodes": [],
        "edges": []
    }))
}

async fn translate(
    Json(req): Json<TranslateRequest>,
) -> (StatusCode, Json<TranslateResponse>) {
    match crate::translator::translate_format(&req.data, &req.from_format, &req.to_format) {
        Ok(result) => (
            StatusCode::OK,
            Json(TranslateResponse {
                result,
                format: req.to_format,
            }),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(TranslateResponse {
                result: format!("Errore: {e}"),
                format: req.from_format,
            }),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_node_status() {
        let app = node_routes();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/node/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_network_health() {
        let app = network_routes();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/network/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_insert_data() {
        let app = data_routes();
        let body = serde_json::json!({
            "content": "test data",
            "domain": "test",
            "source": "unit-test"
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/data")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_node_peers_empty() {
        let app = node_routes();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/node/peers")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
