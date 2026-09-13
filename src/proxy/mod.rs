pub mod forwarder;

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{ConnectInfo, OriginalUri, Request, State};
use axum::http::{header, HeaderName, HeaderValue, Method};
use axum::response::{IntoResponse, Response};

use crate::config::AppConfig;
use crate::error::ProxyError;
use crate::load_balancer::LoadBalancer;

use super::backend::BackendManager;

#[derive(Clone)]
pub struct ProxyState {
    pub config: Arc<AppConfig>,
    pub backend_manager: Arc<BackendManager>,
    pub load_balancer: Arc<dyn LoadBalancer>,
}

pub async fn handler(
    State(state): State<ProxyState>,
    OriginalUri(uri): OriginalUri,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    mut req: Request<Body>,
) -> Result<Response, ProxyError> {
    let method = req.method().clone();

    if method == Method::GET && uri.path() == "/__health" || uri.path() == "/__stats" {
        return Err(ProxyError::Internal("handled by dedicated routes".into()));
    }

    let backend = state
        .load_balancer
        .next_backend()
        .ok_or(ProxyError::NoBackends)?;

    let backend_url = format!("{}{}", backend.url, uri);

    let headers = req.headers().clone();
    let mut proxy_headers = HeaderMap::new();

    if state.config.proxy.add_x_forwarded_for {
        let existing = headers
            .get(header::X_FORWARDED_FOR)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let new_val = if existing.is_empty() {
            addr.ip().to_string()
        } else {
            format!("{existing}, {}", addr.ip())
        };
        if let Ok(val) = HeaderValue::from_str(&new_val) {
            proxy_headers.insert(header::X_FORWARDED_FOR, val);
        }
    }

    if state.config.proxy.add_x_real_ip {
        if let Ok(val) = HeaderValue::from_str(&addr.ip().to_string()) {
            proxy_headers.insert(
                HeaderName::from_static("x-real-ip"),
                val,
            );
        }
    }

    if state.config.proxy.add_via_header {
        if let Ok(val) = HeaderValue::from_str(&state.config.proxy.via_value) {
            proxy_headers.insert(header::VIA, val);
        }
    }

    for header_name in &state.config.proxy.strip_request_headers {
        if let Ok(name) = HeaderName::from_bytes(header_name.as_bytes()) {
            req.headers_mut().remove(&name);
        }
    }

    for (key, value) in &state.config.proxy.add_request_headers {
        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(key.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            req.headers_mut().insert(name, val);
        }
    }

    for (name, value) in &proxy_headers {
        req.headers_mut().insert(name.clone(), value.clone());
    }

    state.backend_manager.increment_connections(&backend.name);
    let start = std::time::Instant::now();

    let result = forwarder::forward_request(
        state.backend_manager.client(),
        &backend_url,
        req,
        std::time::Duration::from_secs(state.config.server.request_timeout_secs),
    )
    .await;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    state
        .backend_manager
        .decrement_connections(&backend.name);

    match result {
        Ok(response) => {
            state.backend_manager.record_request(&backend.name, true, elapsed);
            Ok(response)
        }
        Err(e) => {
            state.backend_manager.record_request(&backend.name, false, elapsed);
            Err(e)
        }
    }
}
