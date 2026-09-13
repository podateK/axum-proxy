pub mod least_connections;
pub mod round_robin;
pub mod weighted;

use std::sync::Arc;

use crate::backend::{Backend, BackendManager};
use crate::config::LoadBalancerConfig;

pub trait LoadBalancer: Send + Sync {
    fn next_backend(&self) -> Option<Backend>;
    fn name(&self) -> &str;
}

pub fn from_config(
    config: &LoadBalancerConfig,
    backend_manager: Arc<BackendManager>,
) -> Arc<dyn LoadBalancer> {
    match config.algorithm.as_str() {
        "least_connections" => {
            Arc::new(least_connections::LeastConnections::new(backend_manager))
        }
        "weighted" => Arc::new(weighted::WeightedRoundRobin::new(backend_manager)),
        _ => Arc::new(round_robin::RoundRobin::new(backend_manager)),
    }
}
