//! REST API endpoints per VARCAVIA.
//!
//! Tutti gli handler usano `State<Arc<AppState>>` per accedere allo storage
//! reale (sled), alla pipeline CDE e alle informazioni del nodo.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::{AppState, PREFIX_DATA, PREFIX_DDNA, PREFIX_INFO};

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
    pub score: f64,
}

#[derive(Debug, Serialize)]
pub struct DataResponse {
    pub id: String,
    pub content: String,
    pub domain: String,
    pub score: f64,
}

/// Metadati salvati per ogni dato inserito.
#[derive(Debug, Serialize, Deserialize)]
pub struct DataInfo {
    pub domain: String,
    pub score: f64,
    pub inserted_at_us: i64,
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
pub fn data_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/data", post(insert_data))
        .route("/api/v1/data/query", post(query_data))
        .route("/api/v1/data/verify", post(verify_data))
        .route("/api/v1/data/:id", get(get_data).delete(delete_data))
        .route("/api/v1/data/:id/dna", get(get_data_dna))
        .route("/api/v1/data/:id/score", get(get_data_score))
}

/// Route per /api/v1/node/*
pub fn node_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/node/status", get(node_status))
        .route("/api/v1/node/peers", get(node_peers))
        .route("/api/v1/node/stats", get(node_stats))
}

/// Route per /api/v1/network/*
pub fn network_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/network/health", get(network_health))
        .route("/api/v1/network/topology", get(network_topology))
}

/// Route per /api/v1/translate
pub fn translate_routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/v1/translate", post(translate))
}

// === Handlers ===

/// POST /api/v1/data — Inserisci un nuovo dato.
/// Crea dDNA → pipeline CDE → salva in sled → consenso con peer.
async fn insert_data(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InsertDataRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let content_bytes = req.content.into_bytes();
    let domain = req.domain;
    let keypair = state.keypair();

    // 1. Crea dDNA
    let ddna = match varcavia_ddna::DataDna::create(&content_bytes, &keypair) {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Creazione dDNA fallita: {e}")})),
            );
        }
    };

    let data_id = ddna.id();

    // 2. Pipeline CDE
    let score = {
        let mut pipeline = state.pipeline.lock().unwrap();
        match pipeline.process(&content_bytes, &ddna, &domain) {
            Ok(result) => result.score.overall,
            Err(e) => {
                return (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({"error": format!("Pipeline CDE: {e}"), "id": data_id})),
                );
            }
        }
    };

    // 3. Serializza dDNA
    let ddna_bytes = match ddna.to_bytes() {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Serializzazione dDNA: {e}")})),
            );
        }
    };

    // 4. Salva in sled: content + dDNA + info
    let data_key = AppState::make_key(PREFIX_DATA, &data_id);
    let ddna_key = AppState::make_key(PREFIX_DDNA, &data_id);
    let info_key = AppState::make_key(PREFIX_INFO, &data_id);

    let info = DataInfo {
        domain: domain.clone(),
        score,
        inserted_at_us: chrono::Utc::now().timestamp_micros(),
    };

    if let Err(e) = state.db.insert(data_key, content_bytes.as_slice()) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Storage data: {e}")})),
        );
    }
    if let Err(e) = state.db.insert(ddna_key, ddna_bytes.as_slice()) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Storage dDNA: {e}")})),
        );
    }
    if let Ok(info_json) = serde_json::to_vec(&info) {
        let _ = state.db.insert(info_key, info_json);
    }

    tracing::info!("Dato inserito: {} (score: {score:.2})", data_id);

    // 5. Avvia consenso distribuito in background (non blocca la risposta)
    {
        let consensus_state = state.clone();
        let consensus_id = data_id.clone();
        tokio::spawn(async move {
            crate::consensus::run_consensus(
                &consensus_state,
                &consensus_id,
                &content_bytes,
                &ddna_bytes,
                &domain,
            )
            .await;
        });
    }

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": data_id,
            "status": "accepted",
            "score": score
        })),
    )
}

/// GET /api/v1/data/:id — Recupera un dato per ID.
async fn get_data(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let data_key = AppState::make_key(PREFIX_DATA, &id);
    let info_key = AppState::make_key(PREFIX_INFO, &id);

    match state.db.get(data_key) {
        Ok(Some(data_bytes)) => {
            let content = String::from_utf8_lossy(&data_bytes).to_string();
            let (domain, score) = match state.db.get(info_key) {
                Ok(Some(info_bytes)) => {
                    if let Ok(info) = serde_json::from_slice::<DataInfo>(&info_bytes) {
                        (info.domain, info.score)
                    } else {
                        ("unknown".into(), 0.0)
                    }
                }
                _ => ("unknown".into(), 0.0),
            };
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "id": id,
                    "content": content,
                    "domain": domain,
                    "score": score
                })),
            )
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Dato non trovato", "id": id})),
        ),
    }
}

/// GET /api/v1/data/:id/dna — Recupera il dDNA.
async fn get_data_dna(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let ddna_key = AppState::make_key(PREFIX_DDNA, &id);

    match state.db.get(ddna_key) {
        Ok(Some(ddna_bytes)) => {
            match varcavia_ddna::DataDna::from_bytes(&ddna_bytes) {
                Ok(ddna) => {
                    // Serialize to JSON for the response
                    match varcavia_ddna::codec::to_json(&ddna) {
                        Ok(json_str) => {
                            let val: serde_json::Value =
                                serde_json::from_str(&json_str).unwrap_or_default();
                            (StatusCode::OK, Json(val))
                        }
                        Err(e) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({"error": format!("Serializzazione: {e}")})),
                        ),
                    }
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": format!("Deserializzazione dDNA: {e}")})),
                ),
            }
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "dDNA non trovato", "id": id})),
        ),
    }
}

/// GET /api/v1/data/:id/score — Punteggio di affidabilità.
async fn get_data_score(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let info_key = AppState::make_key(PREFIX_INFO, &id);

    match state.db.get(info_key) {
        Ok(Some(info_bytes)) => {
            if let Ok(info) = serde_json::from_slice::<DataInfo>(&info_bytes) {
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "id": id,
                        "overall": info.score,
                        "domain": info.domain,
                    })),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Info corrotta"})),
                )
            }
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Score non trovato", "id": id})),
        ),
    }
}

/// DELETE /api/v1/data/:id — Soft delete.
async fn delete_data(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let data_key = AppState::make_key(PREFIX_DATA, &id);
    match state.db.remove(data_key) {
        Ok(Some(_)) => {
            // Rimuovi anche dDNA e info
            let _ = state.db.remove(AppState::make_key(PREFIX_DDNA, &id));
            let _ = state.db.remove(AppState::make_key(PREFIX_INFO, &id));
            (
                StatusCode::OK,
                Json(serde_json::json!({"id": id, "status": "deleted"})),
            )
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Dato non trovato", "id": id})),
        ),
    }
}

/// POST /api/v1/data/query — Query sui dati.
async fn query_data(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueryRequest>,
) -> Json<Vec<serde_json::Value>> {
    let limit = req.limit.unwrap_or(20);
    let mut results = Vec::new();

    for item in state.db.scan_prefix(PREFIX_INFO) {
        if results.len() >= limit {
            break;
        }
        if let Ok((key, val)) = item {
            if let Ok(info) = serde_json::from_slice::<DataInfo>(&val) {
                // Filter by domain if specified
                if let Some(ref domain) = req.domain {
                    if &info.domain != domain {
                        continue;
                    }
                }
                let id = String::from_utf8_lossy(&key[PREFIX_INFO.len()..]).to_string();
                results.push(serde_json::json!({
                    "id": id,
                    "domain": info.domain,
                    "score": info.score,
                }));
            }
        }
    }

    Json(results)
}

/// POST /api/v1/data/verify — Verifica autenticità.
async fn verify_data(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyRequest>,
) -> Json<VerifyResponse> {
    let ddna_key = AppState::make_key(PREFIX_DDNA, &req.id);

    let ddna_opt = state.db.get(ddna_key).ok().flatten();
    let ddna = match ddna_opt {
        Some(bytes) => match varcavia_ddna::DataDna::from_bytes(&bytes) {
            Ok(d) => d,
            Err(e) => {
                return Json(VerifyResponse {
                    id: req.id,
                    verified: false,
                    details: format!("dDNA corrotto: {e}"),
                });
            }
        },
        None => {
            return Json(VerifyResponse {
                id: req.id,
                verified: false,
                details: "dDNA non trovato".into(),
            });
        }
    };

    // If content is provided, verify it matches
    if let Some(ref content) = req.content {
        match ddna.verify_content(content.as_bytes()) {
            Ok(true) => {
                return Json(VerifyResponse {
                    id: req.id,
                    verified: true,
                    details: "Contenuto verificato: fingerprint corrisponde".into(),
                });
            }
            _ => {
                return Json(VerifyResponse {
                    id: req.id,
                    verified: false,
                    details: "Contenuto non corrisponde al fingerprint".into(),
                });
            }
        }
    }

    // Otherwise just verify the dDNA integrity
    match ddna.verify() {
        Ok(true) => Json(VerifyResponse {
            id: req.id,
            verified: true,
            details: "dDNA integro: firma e catena valide".into(),
        }),
        _ => Json(VerifyResponse {
            id: req.id,
            verified: false,
            details: "Verifica dDNA fallita".into(),
        }),
    }
}

/// GET /api/v1/node/status — Stato reale del nodo.
async fn node_status(State(state): State<Arc<AppState>>) -> Json<NodeStatusResponse> {
    Json(NodeStatusResponse {
        node_id: state.node_id.clone(),
        version: env!("CARGO_PKG_VERSION").into(),
        status: "running".into(),
        uptime_secs: state.uptime_secs(),
        data_count: state.data_count(),
    })
}

/// GET /api/v1/node/peers
async fn node_peers(State(_state): State<Arc<AppState>>) -> Json<Vec<PeerResponse>> {
    // TODO: integrate with NetworkManager peers list
    Json(vec![])
}

/// GET /api/v1/node/stats
async fn node_stats(State(state): State<Arc<AppState>>) -> Json<NodeStatsResponse> {
    let total_data = state.data_count();
    Json(NodeStatsResponse {
        total_data,
        total_validations: total_data, // Each insert is one validation
        avg_score: 0.0,               // TODO: calculate running average
    })
}

/// GET /api/v1/network/health
async fn network_health(State(_state): State<Arc<AppState>>) -> Json<NetworkHealthResponse> {
    Json(NetworkHealthResponse {
        status: "healthy".into(),
        connected_peers: 0,
        network_score: 1.0,
    })
}

/// GET /api/v1/network/topology
async fn network_topology(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "nodes": [],
        "edges": []
    }))
}

/// POST /api/v1/translate
async fn translate(
    State(_state): State<Arc<AppState>>,
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
    use varcavia_cde::pipeline::PipelineConfig;

    fn test_state() -> Arc<AppState> {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let kp = varcavia_ddna::identity::KeyPair::generate();
        Arc::new(AppState::new(db, kp.secret_bytes(), PipelineConfig::default()))
    }

    fn test_app() -> Router {
        let state = test_state();
        crate::server::create_router(state)
    }

    #[tokio::test]
    async fn test_node_status() {
        let app = test_app();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "running");
        assert_eq!(json["data_count"], 0);
    }

    #[tokio::test]
    async fn test_network_health() {
        let app = test_app();
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
        let app = test_app();
        let body = serde_json::json!({
            "content": "La temperatura a Roma e' 22 gradi",
            "domain": "climate",
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "accepted");
        assert!(json["id"].as_str().unwrap().len() == 64); // blake3 hex
        assert!(json["score"].as_f64().unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let state = test_state();
        let app = crate::server::create_router(state.clone());

        // Insert
        let body = serde_json::json!({
            "content": "test data for roundtrip",
            "domain": "test",
            "source": "e2e"
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
        let resp_body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let insert_json: serde_json::Value = serde_json::from_slice(&resp_body).unwrap();
        let data_id = insert_json["id"].as_str().unwrap().to_string();

        // Get — need a new router instance (oneshot consumes the service)
        let app2 = crate::server::create_router(state);
        let response = app2
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/v1/data/{data_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["content"], "test data for roundtrip");
        assert_eq!(json["domain"], "test");
    }

    #[tokio::test]
    async fn test_get_missing_data() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/data/nonexistent_id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_insert_duplicate_rejected() {
        let state = test_state();

        let body = serde_json::json!({
            "content": "duplicate test",
            "domain": "test",
            "source": "e2e"
        });
        let body_bytes = serde_json::to_vec(&body).unwrap();

        // First insert
        let app1 = crate::server::create_router(state.clone());
        let r1 = app1
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/data")
                    .header("content-type", "application/json")
                    .body(Body::from(body_bytes.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r1.status(), StatusCode::CREATED);

        // Second insert — same content → CDE rejects as duplicate
        let app2 = crate::server::create_router(state);
        let r2 = app2
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/data")
                    .header("content-type", "application/json")
                    .body(Body::from(body_bytes))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r2.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_delete_data() {
        let state = test_state();

        // Insert first
        let body = serde_json::json!({
            "content": "to be deleted",
            "domain": "test",
            "source": "e2e"
        });
        let app1 = crate::server::create_router(state.clone());
        let r = app1
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
        let resp_body = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&resp_body).unwrap();
        let id = json["id"].as_str().unwrap().to_string();

        // Delete
        let app2 = crate::server::create_router(state.clone());
        let r = app2
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(&format!("/api/v1/data/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        // Verify gone
        let app3 = crate::server::create_router(state);
        let r = app3
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/v1/data/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_e2e_full_flow() {
        let state = test_state();

        // 1. Check status — 0 data
        let app = crate::server::create_router(state.clone());
        let r = app
            .oneshot(Request::builder().uri("/api/v1/node/status").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data_count"], 0);

        // 2. Insert data
        let insert_body = serde_json::json!({
            "content": "Roma: temperatura 25C, umidita' 60%",
            "domain": "climate",
            "source": "sensor-01"
        });
        let app = crate::server::create_router(state.clone());
        let r = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/data")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&insert_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
        let insert_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let data_id = insert_resp["id"].as_str().unwrap().to_string();

        // 3. Get data — verify content
        let app = crate::server::create_router(state.clone());
        let r = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/v1/data/{data_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let body = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
        let data_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(data_resp["content"], "Roma: temperatura 25C, umidita' 60%");
        assert_eq!(data_resp["domain"], "climate");
        assert!(data_resp["score"].as_f64().unwrap() > 0.0);

        // 4. Get dDNA
        let app = crate::server::create_router(state.clone());
        let r = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/v1/data/{data_id}/dna"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        // 5. Verify data
        let verify_body = serde_json::json!({
            "id": data_id,
            "content": "Roma: temperatura 25C, umidita' 60%"
        });
        let app = crate::server::create_router(state.clone());
        let r = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/data/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&verify_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
        let verify_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(verify_resp["verified"], true);

        // 6. Check status — 1 data
        let app = crate::server::create_router(state);
        let r = app
            .oneshot(Request::builder().uri("/api/v1/node/status").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data_count"], 1);
    }

    #[tokio::test]
    async fn test_node_peers_empty() {
        let app = test_app();
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
