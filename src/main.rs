use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::backend::BackendManager;
use crate::backend::health::HealthChecker;
use crate::config::AppConfig;
use crate::load_balancer::LoadBalancer;
use crate::proxy::ProxyState;

mod backend;
mod config;
mod error;
mod load_balancer;
mod middleware;
mod proxy;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "axum_proxy=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::load().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config, using defaults: {e}");
        AppConfig::default()
    });

    let bind_addr = config.server.bind_addr.clone();
    let listener = TcpListener::bind(&bind_addr).await?;
    tracing::info!("Listening on {bind_addr}");

    let backend_manager = Arc::new(BackendManager::new(config.backends.clone()));
    let load_balancer: Arc<dyn LoadBalancer> =
        load_balancer::from_config(&config.load_balancer, backend_manager.clone());
    let health_checker = Arc::new(HealthChecker::new(
        config.health_check.clone(),
        backend_manager.clone(),
    ));

    let state = ProxyState {
        config: Arc::new(config.clone()),
        backend_manager,
        load_balancer,
    };

    let mut app = Router::new()
        .route(
            "/__health",
            axum::routing::get(middleware::health_check::dashboard),
        )
        .route(
            "/__stats",
            axum::routing::get(middleware::health_check::stats),
        )
        .fallback(proxy::handler)
        .with_state(state)
        .layer(middleware::logging::layer())
        .layer(middleware::health_check::cors_layer());

    if config.rate_limit.enabled {
        app = app.layer(middleware::rate_limiter::RateLimiterLayer::new(
            &config.rate_limit,
        ));
    }

    health_checker.spawn();

    let shutdown_signal = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("Shutdown signal received");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    health_checker.shutdown();
    tracing::info!("Server shut down gracefully");
    Ok(())
}
