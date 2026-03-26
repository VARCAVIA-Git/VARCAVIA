//! Middleware — Logging, CORS, rate limiting.
//!
//! La maggior parte del middleware è configurata a livello di router in server.rs
//! usando tower-http. Qui definiamo utilità aggiuntive.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Risposta di errore standard per le API.
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub code: u16,
}

impl ApiError {
    pub fn not_found(msg: &str) -> Self {
        ApiError {
            error: msg.to_string(),
            code: 404,
        }
    }

    pub fn bad_request(msg: &str) -> Self {
        ApiError {
            error: msg.to_string(),
            code: 400,
        }
    }

    pub fn internal(msg: &str) -> Self {
        ApiError {
            error: msg.to_string(),
            code: 500,
        }
    }

    pub fn unauthorized() -> Self {
        ApiError {
            error: "Autenticazione richiesta".to_string(),
            code: 401,
        }
    }

    pub fn rate_limited() -> Self {
        ApiError {
            error: "Rate limit superato".to_string(),
            code: 429,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = axum::Json(self);
        (status, body).into_response()
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
}
