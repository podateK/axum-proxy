use std::sync::Arc;

use crate::backend::{Backend, BackendManager};
use crate::load_balancer::LoadBalancer;

pub struct LeastConnections {
    backend_manager: Arc<BackendManager>,
}

impl LeastConnections {
    pub fn new(backend_manager: Arc<BackendManager>) -> Self {
        Self { backend_manager }
    }
}

impl LoadBalancer for LeastConnections {
    fn next_backend(&self) -> Option<Backend> {
        self.backend_manager
            .get_healthy_backends()
            .into_iter()
            .min_by_key(|b| b.active_connections)
    }

    fn name(&self) -> &str {
        "least_connections"
    }
}
