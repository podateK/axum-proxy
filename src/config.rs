use std::env;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub backends: Vec<BackendConfig>,
    pub load_balancer: LoadBalancerConfig,
    pub health_check: HealthCheckConfig,
    pub rate_limit: RateLimitConfig,
    pub proxy: ProxyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,
    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: u64,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub name: String,
    pub url: String,
    #[serde(default = "default_weight")]
    pub weight: u32,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
    #[serde(default = "default_sticky_sessions")]
    pub sticky_sessions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthCheckConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_interval_secs")]
    pub interval_secs: u64,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_healthy_threshold")]
    pub healthy_threshold: u32,
    #[serde(default = "default_unhealthy_threshold")]
    pub unhealthy_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_rate_limit_requests")]
    pub max_requests: u64,
    #[serde(default = "default_rate_limit_window_secs")]
    pub window_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default = "default_x_forwarded_for")]
    pub add_x_forwarded_for: bool,
    #[serde(default = "default_x_real_ip")]
    pub add_x_real_ip: bool,
    #[serde(default = "default_via_header")]
    pub add_via_header: bool,
    #[serde(default = "default_via_value")]
    pub via_value: String,
    #[serde(default = "default_strip_headers")]
    pub strip_request_headers: Vec<String>,
    #[serde(default)]
    pub add_request_headers: std::collections::HashMap<String, String>,
    #[serde(default = "default_websocket")]
    pub websocket_enabled: bool,
}

fn default_bind_addr() -> String {
    "0.0.0.0:8080".to_string()
}
fn default_request_timeout_secs() -> u64 {
    30
}
fn default_max_connections() -> usize {
    1024
}
fn default_weight() -> u32 {
    1
}
fn default_max_retries() -> u32 {
    3
}
fn default_algorithm() -> String {
    "round_robin".to_string()
}
fn default_sticky_sessions() -> bool {
    false
}
fn default_true() -> bool {
    true
}
fn default_interval_secs() -> u64 {
    10
}
fn default_timeout_secs() -> u64 {
    5
}
fn default_healthy_threshold() -> u32 {
    2
}
fn default_unhealthy_threshold() -> u32 {
    3
}
fn default_rate_limit_requests() -> u64 {
    1000
}
fn default_rate_limit_window_secs() -> u64 {
    60
}
fn default_x_forwarded_for() -> bool {
    true
}
fn default_x_real_ip() -> bool {
    true
}
fn default_via_header() -> bool {
    true
}
fn default_via_value() -> String {
    "axum-proxy".to_string()
}
fn default_strip_headers() -> Vec<String> {
    vec!["proxy-connection".to_string()]
}
fn default_websocket() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                bind_addr: default_bind_addr(),
                request_timeout_secs: default_request_timeout_secs(),
                max_connections: default_max_connections(),
            },
            backends: vec![BackendConfig {
                name: "backend-1".to_string(),
                url: "http://127.0.0.1:3000".to_string(),
                weight: default_weight(),
                max_retries: default_max_retries(),
            }],
            load_balancer: LoadBalancerConfig {
                algorithm: default_algorithm(),
                sticky_sessions: default_sticky_sessions(),
            },
            health_check: HealthCheckConfig::default(),
            rate_limit: RateLimitConfig {
                enabled: default_true(),
                max_requests: default_rate_limit_requests(),
                window_secs: default_rate_limit_window_secs(),
            },
            proxy: ProxyConfig {
                add_x_forwarded_for: default_x_forwarded_for(),
                add_x_real_ip: default_x_real_ip(),
                add_via_header: default_via_header(),
                via_value: default_via_value(),
                strip_request_headers: default_strip_headers(),
                add_request_headers: std::collections::HashMap::new(),
                websocket_enabled: default_websocket(),
            },
        }
    }
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        if let Ok(path) = env::var("PROXY_CONFIG") {
            let content = std::fs::read_to_string(&path)?;
            let config: AppConfig = toml::from_str(&content)?;
            return Ok(config);
        }

        for candidate in &["config.toml", "proxy.toml", ".proxy.toml"] {
            if Path::new(candidate).exists() {
                let content = std::fs::read_to_string(candidate)?;
                let config: AppConfig = toml::from_str(&content)?;
                return Ok(config);
            }
        }

        if let Ok(config_str) = env::var("PROXY_CONFIG_JSON") {
            let config: AppConfig = serde_json::from_str(&config_str)?;
            return Ok(config);
        }

        Ok(AppConfig::default())
    }
}
