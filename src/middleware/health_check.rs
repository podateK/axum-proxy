use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Method, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};

use crate::error::ProxyError;
use crate::proxy::ProxyState;

pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
        .max_age(Duration::from_secs(3600))
}

pub async fn dashboard() -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>axum-proxy Health Dashboard</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: system-ui, sans-serif; background: #0f172a; color: #e2e8f0; padding: 2rem; }
        h1 { font-size: 1.5rem; margin-bottom: 1rem; color: #38bdf8; }
        .card { background: #1e293b; border-radius: 8px; padding: 1rem; margin-bottom: 1rem; border-left: 4px solid #22c55e; }
        .card.unhealthy { border-left-color: #ef4444; }
        .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); gap: 0.5rem; margin-top: 0.5rem; }
        .stat { background: #0f172a; padding: 0.5rem; border-radius: 4px; }
        .stat-label { font-size: 0.75rem; color: #94a3b8; }
        .stat-value { font-size: 1.25rem; font-weight: bold; color: #38bdf8; }
        .status { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 0.5rem; }
        .status.healthy { background: #22c55e; }
        .status.unhealthy { background: #ef4444; }
    </style>
</head>
<body>
    <h1>axum-proxy Health Dashboard</h1>
    <div id="backends"></div>
    <script>
        async function load() {
            try {
                const resp = await fetch('/__stats');
                const data = await resp.json();
                const container = document.getElementById('backends');
                container.innerHTML = data.backends.map(b => `
                    <div class="card ${b.healthy ? '' : 'unhealthy'}">
                        <strong>
                            <span class="status ${b.healthy ? 'healthy' : 'unhealthy'}"></span>
                            ${b.name}
                        </strong>
                        <span style="color:#94a3b8;margin-left:0.5rem">${b.url}</span>
                        <div class="stats">
                            <div class="stat"><div class="stat-label">Active</div><div class="stat-value">${b.active_connections}</div></div>
                            <div class="stat"><div class="stat-label">Total</div><div class="stat-value">${b.total_requests}</div></div>
                            <div class="stat"><div class="stat-label">Failed</div><div class="stat-value">${b.failed_requests}</div></div>
                            <div class="stat"><div class="stat-label">Avg MS</div><div class="stat-value">${b.avg_response_ms.toFixed(1)}</div></div>
                            <div class="stat"><div class="stat-label">Success</div><div class="stat-value">${b.success_rate.toFixed(1)}%</div></div>
                            <div class="stat"><div class="stat-label">Weight</div><div class="stat-value">${b.weight}</div></div>
                        </div>
                    </div>
                `).join('');
            } catch (e) {
                document.getElementById('backends').innerHTML = '<p style="color:#ef4444">Failed to load stats</p>';
            }
        }
        load();
        setInterval(load, 5000);
    </script>
</body>
</html>"#;

    Html(html.to_string())
}

pub async fn stats(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, ProxyError> {
    let stats = state.backend_manager.stats();
    let load_balancer_name = state.load_balancer.name().to_string();

    let response = json!({
        "load_balancer": load_balancer_name,
        "backends": stats,
        "total_backends": stats.len(),
        "healthy_backends": stats.iter().filter(|s| s.healthy).count(),
    });

    Ok(axum::Json(response))
}
