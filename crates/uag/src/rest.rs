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
    #[serde(default)]
    pub verification_count: u32,
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
        .route("/api/v1/node/consensus/:id", get(get_consensus_status))
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

/// Route per /api/v1/metrics
pub fn metrics_routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/v1/metrics", get(metrics))
}

/// Route per ricerca semantica e estrazione fatti
pub fn search_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/search", get(semantic_search))
        .route("/api/v1/extract", post(extract_facts))
}

/// Route batch per uso enterprise
pub fn batch_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/batch/verify", post(batch_verify))
        .route("/api/v1/batch/submit", post(batch_submit))
}

/// Route hero per demo pubblica + health + stats
pub fn hero_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/verify", get(hero_verify))
        .route("/api/v1/stats", get(public_stats))
        .route("/health", get(health_check))
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
        verification_count: 1,
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

    state.facts_ingested.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
async fn node_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "node_id": state.node_id,
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "uptime_secs": state.uptime_secs(),
        "data_count": state.data_count(),
        "avg_latency_ms": (state.avg_latency_ms() * 100.0).round() / 100.0,
    }))
}

/// GET /api/v1/node/peers — Lista peer reale da AppState.
async fn node_peers(State(state): State<Arc<AppState>>) -> Json<Vec<PeerResponse>> {
    let addrs = state.get_peers().await;
    Json(
        addrs
            .iter()
            .enumerate()
            .map(|(i, addr)| PeerResponse {
                node_id: format!("peer-{i}"),
                address: addr.to_string(),
            })
            .collect(),
    )
}

/// GET /api/v1/node/stats
async fn node_stats(State(state): State<Arc<AppState>>) -> Json<NodeStatsResponse> {
    let total_data = state.data_count();
    Json(NodeStatsResponse {
        total_data,
        total_validations: total_data,
        avg_score: 0.0,
    })
}

/// GET /api/v1/node/consensus/:id — Stato del consenso per un dato.
async fn get_consensus_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    use crate::state::PREFIX_CONSENSUS;

    let info_key = AppState::make_key(PREFIX_INFO, &id);
    let ddna_key = AppState::make_key(PREFIX_DDNA, &id);
    let consensus_key = AppState::make_key(PREFIX_CONSENSUS, &id);
    let peers = state.get_peers().await;

    match state.db.get(&info_key) {
        Ok(Some(info_bytes)) => {
            let has_ddna = state.db.get(&ddna_key).ok().flatten().is_some();
            if let Ok(info) = serde_json::from_slice::<DataInfo>(&info_bytes) {
                // Recupera record di consenso se disponibile
                let consensus = state
                    .db
                    .get(&consensus_key)
                    .ok()
                    .flatten()
                    .and_then(|b| {
                        serde_json::from_slice::<crate::consensus::ConsensusRecord>(&b).ok()
                    });

                let (votes, consensus_score, consensus_confirmed, consensus_timestamp) =
                    if let Some(ref cr) = consensus {
                        let votes: Vec<serde_json::Value> = cr
                            .votes
                            .iter()
                            .map(|v| {
                                serde_json::json!({
                                    "node_id": &v.node_id[..16.min(v.node_id.len())],
                                    "vote": v.vote,
                                    "confidence": v.confidence,
                                })
                            })
                            .collect();
                        (votes, cr.score, cr.confirmed, Some(cr.timestamp_us))
                    } else {
                        (vec![], 0.0, false, None)
                    };

                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "id": id,
                        "score": info.score,
                        "domain": info.domain,
                        "has_ddna": has_ddna,
                        "peer_count": peers.len(),
                        "consensus_possible": !peers.is_empty(),
                        "consensus": {
                            "score": consensus_score,
                            "confirmed": consensus_confirmed,
                            "votes_received": votes.len(),
                            "votes": votes,
                            "timestamp_us": consensus_timestamp,
                        },
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
            Json(serde_json::json!({"error": "Dato non trovato", "id": id})),
        ),
    }
}

/// GET /api/v1/network/health — Salute rete con peer count reale.
async fn network_health(State(state): State<Arc<AppState>>) -> Json<NetworkHealthResponse> {
    let peers = state.get_peers().await;
    Json(NetworkHealthResponse {
        status: if peers.is_empty() { "standalone" } else { "healthy" }.into(),
        connected_peers: peers.len(),
        network_score: if peers.is_empty() { 0.5 } else { 1.0 },
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

/// GET /api/v1/verify?fact=... — Hero endpoint per demo pubblica.
/// Accetta un fatto come query string, lo inserisce nel sistema,
/// e restituisce Data DNA + score in un unico response.
async fn hero_verify(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let fact = match params.get("fact") {
        Some(f) if !f.trim().is_empty() => f.trim().to_string(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Parametro 'fact' mancante. Uso: /api/v1/verify?fact=Earth+diameter+is+12742+km"
                })),
            );
        }
    };

    let content_bytes = fact.as_bytes();
    let keypair = state.keypair();

    // Crea dDNA
    let ddna = match varcavia_ddna::DataDna::create(content_bytes, &keypair) {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("dDNA creation failed: {e}")})),
            );
        }
    };

    let data_id = ddna.id();
    let fingerprint_blake3 = hex::encode(ddna.fingerprint.blake3);
    let fingerprint_sha3 = hex::encode(&ddna.fingerprint.sha3_512[..32]);

    // Cross-check: cerca fatti simili nel DB
    let (related_facts, cross_check_score) = find_related_facts(&state, &fact);

    let dna_json = serde_json::json!({
        "id": data_id,
        "fingerprint": {
            "blake3": fingerprint_blake3,
            "sha3_512": fingerprint_sha3,
        },
        "source": {
            "public_key": hex::encode(ddna.source.public_key),
            "identity_type": "Pseudonymous",
            "reputation": ddna.source.reputation_score,
        },
        "temporal": {
            "timestamp_us": ddna.temporal.timestamp_us,
            "clock_source": "System",
            "precision_us": ddna.temporal.precision_us,
        },
        "version": ddna.version,
    });

    // Pipeline CDE
    let (score, warnings) = {
        let mut pipeline = state.pipeline.lock().unwrap();
        match pipeline.process(content_bytes, &ddna, "general") {
            Ok(result) => (result.score.overall, result.warnings),
            Err(e) => {
                // Duplicato esatto — gia nel DB
                state.inc_verifications();
                let info_key = AppState::make_key(PREFIX_INFO, &data_id);
                let (existing_score, vcount) = match state.db.get(&info_key) {
                    Ok(Some(b)) => {
                        if let Ok(mut info) = serde_json::from_slice::<DataInfo>(&b) {
                            info.verification_count += 1;
                            let sc = info.score;
                            let vc = info.verification_count;
                            if let Ok(j) = serde_json::to_vec(&info) {
                                let _ = state.db.insert(&info_key, j);
                            }
                            (sc, vc)
                        } else { (0.0, 1) }
                    }
                    _ => (0.0, 1),
                };
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "fact": fact,
                        "status": "already_certified",
                        "message": "Previously certified — this fact was already in the network.",
                        "data_dna": dna_json,
                        "score": existing_score,
                        "cross_check_score": cross_check_score,
                        "related_facts": related_facts,
                        "verification_count": vcount,
                        "duplicate": true,
                        "note": format!("{e}"),
                    })),
                );
            }
        }
    };

    // Salva il nuovo fatto
    state.inc_verifications();
    state.facts_ingested.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if let Ok(ddna_bytes) = ddna.to_bytes() {
        let data_key = AppState::make_key(PREFIX_DATA, &data_id);
        let ddna_key = AppState::make_key(PREFIX_DDNA, &data_id);
        let info_key = AppState::make_key(PREFIX_INFO, &data_id);
        let _ = state.db.insert(data_key, content_bytes);
        let _ = state.db.insert(ddna_key, ddna_bytes.as_slice());
        let info = DataInfo {
            domain: "general".into(),
            score,
            inserted_at_us: chrono::Utc::now().timestamp_micros(),
            verification_count: 1,
        };
        if let Ok(j) = serde_json::to_vec(&info) {
            let _ = state.db.insert(info_key, j);
        }
    }

    // Determina status in base al cross-check
    let (status, message) = if cross_check_score >= 0.5 {
        // Fatto simile trovato — segnala
        ("similar_found", format!(
            "Similar fact found in the network ({}% match). Cryptographic identity issued — this certifies provenance, not factual accuracy.",
            (cross_check_score * 100.0).round()
        ))
    } else {
        ("certified", "Certified — this fact has been cryptographically stamped with a Data DNA. This certifies its identity and provenance, not its factual accuracy.".to_string())
    };

    let mut certify_warnings = warnings;
    if cross_check_score < 0.1 {
        certify_warnings.push("No supporting facts found in the network".into());
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "fact": fact,
            "status": status,
            "message": message,
            "data_dna": dna_json,
            "score": score,
            "cross_check_score": cross_check_score,
            "related_facts": related_facts,
            "verification_count": 1,
            "warnings": certify_warnings,
            "duplicate": false,
        })),
    )
}

/// Cerca fatti correlati nel DB per cross-check.
/// Restituisce (related_facts_json, best_similarity).
fn find_related_facts(state: &AppState, query: &str) -> (Vec<serde_json::Value>, f64) {
    let mut scored: Vec<(String, String, f64)> = Vec::new();

    for (key, val) in state.db.scan_prefix(PREFIX_DATA).flatten() {
        let id = String::from_utf8_lossy(&key[PREFIX_DATA.len()..]).to_string();
        let content = String::from_utf8_lossy(&val).to_string();
        let sim = varcavia_cde::dedup::text_similarity(query, &content);
        if sim > 0.05 {
            scored.push((id, content, sim));
        }
    }

    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(5);

    let cross_check_score = scored.first().map(|(_, _, s)| *s).unwrap_or(0.0);

    let related: Vec<serde_json::Value> = scored
        .iter()
        .filter(|(_, _, s)| *s > 0.1)
        .map(|(id, content, sim)| {
            serde_json::json!({
                "id": id,
                "content": content,
                "similarity": (sim * 1000.0).round() / 1000.0,
            })
        })
        .collect();

    (related, (cross_check_score * 1000.0).round() / 1000.0)
}

/// GET /api/v1/search?q=...&limit=5 — Ricerca semantica per similarita trigram.
async fn semantic_search(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let query = match params.get("q") {
        Some(q) if !q.trim().is_empty() => q.trim().to_string(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Parametro 'q' mancante. Uso: /api/v1/search?q=Rome+temperature"})),
            );
        }
    };
    let limit: usize = params
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(5);

    let mut scored: Vec<(String, String, f64, f64)> = Vec::new(); // (id, content, similarity, score)

    for (key, val) in state.db.scan_prefix(PREFIX_DATA).flatten() {
        let id = String::from_utf8_lossy(&key[PREFIX_DATA.len()..]).to_string();
        let content = String::from_utf8_lossy(&val).to_string();
        let sim = varcavia_cde::dedup::text_similarity(&query, &content);
        if sim > 0.05 {
            let info_key = AppState::make_key(PREFIX_INFO, &id);
            let data_score = state
                .db
                .get(info_key)
                .ok()
                .flatten()
                .and_then(|b| serde_json::from_slice::<DataInfo>(&b).ok())
                .map(|i| i.score)
                .unwrap_or(0.0);
            scored.push((id, content, sim, data_score));
        }
    }

    // Ordina per similarita decrescente
    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    let results: Vec<serde_json::Value> = scored
        .iter()
        .map(|(id, content, sim, score)| {
            serde_json::json!({
                "id": id,
                "content": content,
                "similarity": (sim * 1000.0).round() / 1000.0,
                "score": score,
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "query": query,
            "results": results,
            "total": results.len(),
        })),
    )
}

/// POST /api/v1/extract — Estrae fatti da un testo lungo e li inserisce.
async fn extract_facts(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let text = match body.get("text").and_then(|v| v.as_str()) {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Campo 'text' mancante"})),
            );
        }
    };
    let domain = body
        .get("domain")
        .and_then(|v| v.as_str())
        .unwrap_or("general")
        .to_string();

    let claims = varcavia_cde::pipeline::extract_claims(&text);
    if claims.is_empty() {
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "claims": [],
                "total": 0,
                "inserted": 0,
                "message": "Nessun fatto estratto dal testo",
            })),
        );
    }

    let keypair = state.keypair();
    let mut results = Vec::new();
    let mut inserted = 0u32;

    for claim in &claims {
        let content_bytes = claim.as_bytes();
        let ddna = match varcavia_ddna::DataDna::create(content_bytes, &keypair) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let data_id = ddna.id();

        // Pipeline CDE
        let score = {
            let mut pipeline = state.pipeline.lock().unwrap();
            match pipeline.process(content_bytes, &ddna, &domain) {
                Ok(result) => result.score.overall,
                Err(_) => {
                    // Duplicato o errore — includi comunque nella lista
                    results.push(serde_json::json!({
                        "claim": claim,
                        "id": data_id,
                        "status": "duplicate",
                    }));
                    continue;
                }
            }
        };

        // Salva
        if let Ok(ddna_bytes) = ddna.to_bytes() {
            let _ = state.db.insert(AppState::make_key(PREFIX_DATA, &data_id), content_bytes);
            let _ = state.db.insert(AppState::make_key(PREFIX_DDNA, &data_id), ddna_bytes.as_slice());
            let info = DataInfo {
                domain: domain.clone(),
                score,
                inserted_at_us: chrono::Utc::now().timestamp_micros(),
                verification_count: 1,
            };
            if let Ok(j) = serde_json::to_vec(&info) {
                let _ = state.db.insert(AppState::make_key(PREFIX_INFO, &data_id), j);
            }
        }
        inserted += 1;
        results.push(serde_json::json!({
            "claim": claim,
            "id": data_id,
            "score": score,
            "status": "inserted",
        }));
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "claims": results,
            "total": claims.len(),
            "inserted": inserted,
        })),
    )
}

/// GET /api/v1/metrics — Metriche operative del nodo.
/// POST /api/v1/batch/verify — Verifica multipli fatti in parallelo.
async fn batch_verify(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let facts = match body.get("facts").and_then(|v| v.as_array()) {
        Some(arr) => arr.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Campo 'facts' (array) mancante"})),
            );
        }
    };

    let mut results = Vec::new();
    for fact_val in &facts {
        let fact = match fact_val.as_str() {
            Some(f) if !f.trim().is_empty() => f.trim(),
            _ => continue,
        };

        let content_bytes = fact.as_bytes();
        let keypair = state.keypair();
        let ddna = match varcavia_ddna::DataDna::create(content_bytes, &keypair) {
            Ok(d) => d,
            Err(e) => {
                results.push(serde_json::json!({"fact": fact, "error": e.to_string()}));
                continue;
            }
        };

        let data_id = ddna.id();
        let fingerprint_blake3 = hex::encode(ddna.fingerprint.blake3);

        let (score, duplicate) = {
            let mut pipeline = state.pipeline.lock().unwrap();
            match pipeline.process(content_bytes, &ddna, "general") {
                Ok(result) => (result.score.overall, false),
                Err(_) => {
                    // Duplicato — recupera score esistente
                    let info_key = AppState::make_key(PREFIX_INFO, &data_id);
                    let existing_score = state.db.get(&info_key).ok().flatten()
                        .and_then(|b| serde_json::from_slice::<DataInfo>(&b).ok())
                        .map(|i| i.score)
                        .unwrap_or(0.0);
                    (existing_score, true)
                }
            }
        };

        if !duplicate {
            state.inc_verifications();
            state.facts_ingested.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if let Ok(ddna_bytes) = ddna.to_bytes() {
                let _ = state.db.insert(AppState::make_key(PREFIX_DATA, &data_id), content_bytes);
                let _ = state.db.insert(AppState::make_key(PREFIX_DDNA, &data_id), ddna_bytes.as_slice());
                let info = DataInfo {
                    domain: "general".into(),
                    score,
                    inserted_at_us: chrono::Utc::now().timestamp_micros(),
                    verification_count: 1,
                };
                if let Ok(j) = serde_json::to_vec(&info) {
                    let _ = state.db.insert(AppState::make_key(PREFIX_INFO, &data_id), j);
                }
            }
        }

        results.push(serde_json::json!({
            "fact": fact,
            "id": data_id,
            "blake3": fingerprint_blake3,
            "score": score,
            "duplicate": duplicate,
        }));
    }

    (StatusCode::OK, Json(serde_json::json!({"results": results, "total": results.len()})))
}

/// POST /api/v1/batch/submit — Inserisce multipli dati in batch.
async fn batch_submit(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let items = match body.get("items").and_then(|v| v.as_array()) {
        Some(arr) => arr.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Campo 'items' (array) mancante. Ogni item: {content, domain, source}"})),
            );
        }
    };

    let keypair = state.keypair();
    let mut inserted = 0u32;
    let mut duplicates = 0u32;
    let mut errors = 0u32;

    for item in &items {
        let content = match item.get("content").and_then(|v| v.as_str()) {
            Some(c) if !c.trim().is_empty() => c.trim(),
            _ => { errors += 1; continue; }
        };
        let domain = item.get("domain").and_then(|v| v.as_str()).unwrap_or("general");
        let content_bytes = content.as_bytes();

        let ddna = match varcavia_ddna::DataDna::create(content_bytes, &keypair) {
            Ok(d) => d,
            Err(_) => { errors += 1; continue; }
        };
        let data_id = ddna.id();

        let score = {
            let mut pipeline = state.pipeline.lock().unwrap();
            match pipeline.process(content_bytes, &ddna, domain) {
                Ok(result) => result.score.overall,
                Err(_) => { duplicates += 1; continue; }
            }
        };

        if let Ok(ddna_bytes) = ddna.to_bytes() {
            let _ = state.db.insert(AppState::make_key(PREFIX_DATA, &data_id), content_bytes);
            let _ = state.db.insert(AppState::make_key(PREFIX_DDNA, &data_id), ddna_bytes.as_slice());
            let info = DataInfo {
                domain: domain.to_string(),
                score,
                inserted_at_us: chrono::Utc::now().timestamp_micros(),
                verification_count: 1,
            };
            if let Ok(j) = serde_json::to_vec(&info) {
                let _ = state.db.insert(AppState::make_key(PREFIX_INFO, &data_id), j);
            }
        }
        state.facts_ingested.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        inserted += 1;
    }

    (StatusCode::OK, Json(serde_json::json!({
        "total": items.len(),
        "inserted": inserted,
        "duplicates": duplicates,
        "errors": errors,
    })))
}

async fn metrics(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let uptime = state.uptime_secs();
    let total_verifications = state.total_verifications.load(std::sync::atomic::Ordering::Relaxed);
    let claims_per_second = if uptime > 0 {
        total_verifications as f64 / uptime as f64
    } else {
        0.0
    };
    let storage_bytes = state.db.size_on_disk().unwrap_or(0);

    let facts_ingested = state.facts_ingested.load(std::sync::atomic::Ordering::Relaxed);

    Json(serde_json::json!({
        "claims_per_second": (claims_per_second * 100.0).round() / 100.0,
        "avg_latency_ms": (state.avg_latency_ms() * 100.0).round() / 100.0,
        "total_verifications": total_verifications,
        "facts_ingested_total": facts_ingested,
        "uptime_hours": (uptime as f64 / 3600.0 * 100.0).round() / 100.0,
        "storage_bytes": storage_bytes,
    }))
}

/// GET /api/v1/stats — Statistiche pubbliche del nodo.
async fn public_stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let peers = state.get_peers().await;
    Json(serde_json::json!({
        "total_data": state.data_count(),
        "total_verifications": state.total_verifications.load(std::sync::atomic::Ordering::Relaxed),
        "uptime_secs": state.uptime_secs(),
        "node_count": 1 + peers.len(),
        "avg_score": compute_avg_score(&state.db),
    }))
}

fn compute_avg_score(db: &sled::Db) -> f64 {
    let mut sum = 0.0;
    let mut count = 0u64;
    for (_, val) in db.scan_prefix(PREFIX_INFO).flatten() {
        if let Ok(info) = serde_json::from_slice::<DataInfo>(&val) {
            sum += info.score;
            count += 1;
        }
    }
    if count == 0 { 0.0 } else { sum / count as f64 }
}

/// GET /health — Health check per Docker/Kubernetes.
async fn health_check(State(state): State<Arc<AppState>>) -> (StatusCode, Json<serde_json::Value>) {
    let ok = state.db.was_recovered() || state.uptime_secs() > 0;
    (
        if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE },
        Json(serde_json::json!({
            "status": if ok { "healthy" } else { "unhealthy" },
            "uptime_secs": state.uptime_secs(),
            "data_count": state.data_count(),
        })),
    )
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

    #[tokio::test]
    async fn test_hero_verify() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/verify?fact=Earth+diameter+is+12742+km")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["fact"], "Earth diameter is 12742 km");
        assert_eq!(json["status"], "certified");
        assert!(json["data_dna"]["id"].as_str().unwrap().len() == 64);
        assert!(json["score"].as_f64().unwrap() > 0.0);
        assert_eq!(json["duplicate"], false);
        assert!(json["message"].as_str().unwrap().contains("not its factual accuracy"));
        assert!(json["cross_check_score"].is_number());
    }

    #[tokio::test]
    async fn test_hero_verify_duplicate() {
        let state = test_state();

        // First call
        let app1 = crate::server::create_router(state.clone());
        let r = app1
            .oneshot(Request::builder().uri("/api/v1/verify?fact=test+fact").body(Body::empty()).unwrap())
            .await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        // Second call — same fact → already_certified
        let app2 = crate::server::create_router(state);
        let r = app2
            .oneshot(Request::builder().uri("/api/v1/verify?fact=test+fact").body(Body::empty()).unwrap())
            .await.unwrap();
        let body = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "already_certified");
        assert_eq!(json["duplicate"], true);
        assert!(json["message"].as_str().unwrap().contains("Previously certified"));
    }

    #[tokio::test]
    async fn test_metrics() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["claims_per_second"].is_number());
        assert!(json["total_verifications"].is_number());
        assert!(json["uptime_hours"].is_number());
        assert!(json["storage_bytes"].is_number());
    }

    #[tokio::test]
    async fn test_hero_verify_missing_param() {
        let app = test_app();
        let response = app
            .oneshot(Request::builder().uri("/api/v1/verify").body(Body::empty()).unwrap())
            .await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_search_endpoint() {
        let state = test_state();

        // Insert some data first
        let body = serde_json::json!({
            "content": "Earth has a radius of 6371 kilometres",
            "domain": "science",
            "source": "test"
        });
        let app = crate::server::create_router(state.clone());
        let _ = app
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

        // Search
        let app2 = crate::server::create_router(state);
        let response = app2
            .oneshot(
                Request::builder()
                    .uri("/api/v1/search?q=Earth+radius&limit=3")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["results"].as_array().unwrap().len() >= 1);
        assert!(json["results"][0]["similarity"].as_f64().unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_search_missing_param() {
        let app = test_app();
        let response = app
            .oneshot(Request::builder().uri("/api/v1/search").body(Body::empty()).unwrap())
            .await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_extract_endpoint() {
        let app = test_app();
        let body = serde_json::json!({
            "text": "Earth is the third planet from the Sun. It has a radius of 6371 km. The weather is nice today.",
            "domain": "science"
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/extract")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["total"].as_u64().unwrap() >= 1);
    }
}
