//! Middleware — Logging, CORS, rate limiting.
//!
//! Rate limiter: sliding-window counter basato su AtomicU64.
//! Nessuna dipendenza esterna — puro Rust.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Risposta di errore standard per le API.
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub code: u16,
}

impl ApiError {
    pub fn not_found(msg: &str) -> Self {
        ApiError { error: msg.to_string(), code: 404 }
    }

    pub fn bad_request(msg: &str) -> Self {
        ApiError { error: msg.to_string(), code: 400 }
    }

    pub fn internal(msg: &str) -> Self {
        ApiError { error: msg.to_string(), code: 500 }
    }

    pub fn unauthorized() -> Self {
        ApiError { error: "Autenticazione richiesta".to_string(), code: 401 }
    }

    pub fn rate_limited() -> Self {
        ApiError { error: "Rate limit superato (max 100 req/s)".to_string(), code: 429 }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = axum::Json(self);
        (status, body).into_response()
    }
}

/// Rate limiter globale — finestra di 1 secondo, max N richieste.
#[derive(Clone)]
pub struct RateLimiter {
    /// Contatore richieste nella finestra corrente
    count: Arc<AtomicU64>,
    /// Secondo corrente (Unix epoch seconds)
    current_sec: Arc<AtomicU64>,
    /// Limite massimo per secondo
    max_per_sec: u64,
}

impl RateLimiter {
    /// Crea un nuovo rate limiter.
    pub fn new(max_per_sec: u64) -> Self {
        RateLimiter {
            count: Arc::new(AtomicU64::new(0)),
            current_sec: Arc::new(AtomicU64::new(0)),
            max_per_sec,
        }
    }

    /// Verifica se una richiesta è permessa. Restituisce true se OK.
    pub fn check(&self) -> bool {
        let now_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let prev_sec = self.current_sec.load(Ordering::Relaxed);
        if now_sec != prev_sec {
            // Nuova finestra — reset counter
            self.current_sec.store(now_sec, Ordering::Relaxed);
            self.count.store(1, Ordering::Relaxed);
            return true;
        }

        let prev_count = self.count.fetch_add(1, Ordering::Relaxed);
        prev_count < self.max_per_sec
    }
}

/// Axum middleware layer per rate limiting.
pub async fn rate_limit_middleware(
    axum::extract::State(limiter): axum::extract::State<RateLimiter>,
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    if !limiter.check() {
        return ApiError::rate_limited().into_response();
    }
    next.run(request).await
}

/// Middleware per tracciare la latenza delle richieste.
pub async fn latency_middleware(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::state::AppState>>,
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    let start = std::time::Instant::now();
    let response = next.run(request).await;
    let elapsed_us = start.elapsed().as_micros() as u64;
    state.record_latency_us(elapsed_us);
    response
}

/// Middleware per autenticazione API key opzionale.
/// Se VARCAVIA_API_KEY è impostata, richiede X-API-Key header per POST/PUT/DELETE.
/// GET rimangono pubblici. Se non configurata, tutto è aperto.
pub async fn api_key_middleware(
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    let expected = std::env::var("VARCAVIA_API_KEY").ok();

    // Se nessuna API key configurata, tutto è aperto
    let Some(expected_key) = expected else {
        return next.run(request).await;
    };

    if expected_key.is_empty() {
        return next.run(request).await;
    }

    // GET e OPTIONS sono sempre pubblici
    let method = request.method().clone();
    if method == axum::http::Method::GET || method == axum::http::Method::OPTIONS {
        return next.run(request).await;
    }

    // Controlla X-API-Key header
    let provided = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    match provided {
        Some(key) if key == expected_key => next.run(request).await,
        _ => ApiError::unauthorized().into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = ApiError::not_found("test");
        assert_eq!(err.code, 404);
    }

    #[test]
    fn test_bad_request_error() {
        let err = ApiError::bad_request("invalid input");
        assert_eq!(err.code, 400);
    }

    #[test]
    fn test_internal_error() {
        let err = ApiError::internal("something went wrong");
        assert_eq!(err.code, 500);
    }

    #[test]
    fn test_unauthorized() {
        let err = ApiError::unauthorized();
        assert_eq!(err.code, 401);
    }

    #[test]
    fn test_rate_limited() {
        let err = ApiError::rate_limited();
        assert_eq!(err.code, 429);
    }

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(100);
        for _ in 0..100 {
            assert!(limiter.check());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(5);
        for _ in 0..5 {
            assert!(limiter.check());
        }
        // 6th should be blocked
        assert!(!limiter.check());
    }

    #[test]
    fn test_rate_limiter_resets_on_new_second() {
        let limiter = RateLimiter::new(2);
        assert!(limiter.check());
        assert!(limiter.check());
        assert!(!limiter.check()); // blocked

        // Simulate time advancing by forcing current_sec to 0
        limiter.current_sec.store(0, Ordering::Relaxed);
        assert!(limiter.check()); // new window
    }
}
