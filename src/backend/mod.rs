use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::BackendConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backend {
    pub name: String,
    pub url: String,
    pub weight: u32,
    pub max_retries: u32,
    pub healthy: bool,
    pub active_connections: u64,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub avg_response_ms: f64,
    pub last_health_check: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendStats {
    pub name: String,
    pub url: String,
    pub healthy: bool,
    pub active_connections: u64,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub avg_response_ms: f64,
    pub success_rate: f64,
    pub weight: u32,
}

pub struct BackendManager {
    backends: DashMap<String, Backend>,
    client: Client,
}

impl BackendManager {
    pub fn new(configs: Vec<BackendConfig>) -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(64)
            .pool_idle_timeout(Duration::from_secs(90))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .tcp_keepalive(Duration::from_secs(30))
            .http2_adaptive_window(true)
            .build()
            .expect("failed to create HTTP client");

        let backends = DashMap::new();
        for cfg in configs {
            backends.insert(
                cfg.name.clone(),
                Backend {
                    name: cfg.name,
                    url: cfg.url,
                    weight: cfg.weight,
                    max_retries: cfg.max_retries,
                    healthy: true,
                    active_connections: 0,
                    total_requests: 0,
                    failed_requests: 0,
                    avg_response_ms: 0.0,
                    last_health_check: None,
                    consecutive_failures: 0,
                },
            );
        }

        Self { backends, client }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn get_healthy_backends(&self) -> Vec<Backend> {
        self.backends
            .iter()
            .filter(|b| b.healthy)
            .map(|b| b.value().clone())
            .collect()
    }

    pub fn get_all_backends(&self) -> Vec<Backend> {
        self.backends.iter().map(|b| b.value().clone()).collect()
    }

    pub fn get_backend(&self, name: &str) -> Option<Backend> {
        self.backends.get(name).map(|b| b.value().clone())
    }

    pub fn set_healthy(&self, name: &str, healthy: bool) {
        if let Some(mut backend) = self.backends.get_mut(name) {
            backend.healthy = healthy;
            backend.last_health_check = Some(Utc::now());
            if healthy {
                backend.consecutive_failures = 0;
            }
        }
    }

    pub fn increment_consecutive_failures(&self, name: &str) -> u32 {
        self.backends
            .get_mut(name)
            .map(|mut b| {
                b.consecutive_failures += 1;
                b.consecutive_failures
            })
            .unwrap_or(0)
    }

    pub fn reset_consecutive_failures(&self, name: &str) -> u32 {
        self.backends
            .get_mut(name)
            .map(|mut b| {
                b.consecutive_failures = 0;
                b.consecutive_failures
            })
            .unwrap_or(0)
    }

    pub fn record_request(&self, name: &str, success: bool, response_ms: f64) {
        if let Some(mut backend) = self.backends.get_mut(name) {
            backend.total_requests += 1;
            if !success {
                backend.failed_requests += 1;
            }
            let n = backend.total_requests as f64;
            backend.avg_response_ms = ((n - 1.0) * backend.avg_response_ms + response_ms) / n;
        }
    }

    pub fn increment_connections(&self, name: &str) {
        if let Some(mut backend) = self.backends.get_mut(name) {
            backend.active_connections += 1;
        }
    }

    pub fn decrement_connections(&self, name: &str) {
        if let Some(mut backend) = self.backends.get_mut(name) {
            backend.active_connections = backend.active_connections.saturating_sub(1);
        }
    }

    pub fn stats(&self) -> Vec<BackendStats> {
        self.backends
            .iter()
            .map(|b| {
                let total = b.total_requests;
                let failed = b.failed_requests;
                let success_rate = if total > 0 {
                    ((total - failed) as f64 / total as f64) * 100.0
                } else {
                    100.0
                };
                BackendStats {
                    name: b.name.clone(),
                    url: b.url.clone(),
                    healthy: b.healthy,
                    active_connections: b.active_connections,
                    total_requests: b.total_requests,
                    failed_requests: b.failed_requests,
                    avg_response_ms: b.avg_response_ms,
                    success_rate,
                    weight: b.weight,
                }
            })
            .collect()
    }
}

pub type SharedBackendManager = Arc<BackendManager>;
