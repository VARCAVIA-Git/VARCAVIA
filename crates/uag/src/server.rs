//! Axum HTTP server per il Universal Access Gateway.

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::rest;

/// Configurazione del server UAG.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub cors_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            cors_origins: vec!["http://localhost:5173".into()],
        }
    }
}

/// Crea il router Axum con tutti gli endpoint configurati.
pub fn create_router() -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .merge(rest::data_routes())
        .merge(rest::node_routes())
        .merge(rest::network_routes())
        .merge(rest::translate_routes())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

/// Avvia il server HTTP.
pub async fn run(config: ServerConfig) -> anyhow::Result<()> {
    let app = create_router();
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!("UAG server avviato su {}", config.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_addr.port(), 8080);
    }

    #[test]
    fn test_create_router() {
        let _router = create_router();
        // Il router si crea senza panic
    }

    #[test]
    fn test_custom_config() {
        let config = ServerConfig {
            bind_addr: "0.0.0.0:9090".parse().unwrap(),
            cors_origins: vec!["http://example.com".into()],
        };
        assert_eq!(config.bind_addr.port(), 9090);
    }
}
