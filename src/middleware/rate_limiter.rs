use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{ConnectInfo, Request};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use tokio::sync::RwLock;
use tower::{Layer, Service};

use crate::config::RateLimitConfig;

struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: u64, window_secs: u64) -> Self {
        let refill_rate = max_tokens as f64 / window_secs as f64;
        Self {
            tokens: max_tokens as f64,
            max_tokens: max_tokens as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }
}

#[derive(Clone)]
struct RateLimiterInner {
    buckets: Arc<DashMap<IpAddr, Arc<RwLock<TokenBucket>>>>,
    max_requests: u64,
    window_secs: u64,
}

#[derive(Clone)]
pub struct RateLimiterLayer {
    inner: RateLimiterInner,
}

impl RateLimiterLayer {
    pub fn new(config: &RateLimitConfig) -> Self {
        Self {
            inner: RateLimiterInner {
                buckets: Arc::new(DashMap::new()),
                max_requests: config.max_requests,
                window_secs: config.window_secs,
            },
        }
    }
}

impl<S> Layer<S> for RateLimiterLayer {
    type Service = RateLimiterService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimiterService {
            inner,
            state: self.inner.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimiterService<S> {
    inner: S,
    state: RateLimiterInner,
}

impl<S> Service<Request<Body>> for RateLimiterService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Send + Clone + 'static,
    S::Future: Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let ip = extract_client_ip(&req);
        let state = self.state.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let bucket = state
                .buckets
                .entry(ip)
                .or_insert_with(|| {
                    Arc::new(RwLock::new(TokenBucket::new(
                        state.max_requests,
                        state.window_secs,
                    )))
                })
                .clone();

            let allowed = {
                let mut bucket = bucket.write().await;
                bucket.try_consume()
            };

            if !allowed {
                tracing::warn!(ip = %ip, "Rate limit exceeded");
                let response = Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .header("Retry-After", state.window_secs.to_string())
                    .body(Body::from("Rate limit exceeded"))
                    .unwrap();
                return Ok(response);
            }

            inner.call(req).await
        })
    }
}

fn extract_client_ip(req: &Request<Body>) -> IpAddr {
    req.extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or_else(|| {
            req.headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next())
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or_else(|| "0.0.0.0".parse().unwrap())
        })
}

pub fn layer(config: &RateLimitConfig) -> Option<RateLimiterLayer> {
    if config.enabled {
        Some(RateLimiterLayer::new(config))
    } else {
        None
    }
}
