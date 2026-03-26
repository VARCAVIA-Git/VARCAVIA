//! Axum HTTP server per il Universal Access Gateway.

use axum::Router;
use axum::response::Html;
use axum::routing::get as get_route;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::rest;
use crate::state::AppState;

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

/// Crea il router Axum con tutti gli endpoint e lo stato condiviso.
pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get_route(landing_page))
        .merge(rest::data_routes())
        .merge(rest::node_routes())
        .merge(rest::network_routes())
        .merge(rest::translate_routes())
        .merge(rest::hero_routes())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

/// Avvia il server HTTP con lo stato condiviso.
pub async fn run(config: ServerConfig, state: Arc<AppState>) -> anyhow::Result<()> {
    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!("UAG server avviato su {}", config.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

/// Serve la landing page HTML.
async fn landing_page() -> Html<&'static str> {
    Html(include_str!("../../../web/public/index.html"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use varcavia_cde::pipeline::PipelineConfig;
    use varcavia_ddna::identity::KeyPair;

    fn test_state() -> Arc<AppState> {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let kp = KeyPair::generate();
        Arc::new(AppState::new(db, kp.secret_bytes(), PipelineConfig::default()))
    }

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_addr.port(), 8080);
    }

    #[test]
    fn test_create_router() {
        let state = test_state();
        let _router = create_router(state);
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
