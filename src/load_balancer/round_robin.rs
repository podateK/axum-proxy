use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::backend::{Backend, BackendManager};
use crate::load_balancer::LoadBalancer;

pub struct RoundRobin {
    backend_manager: Arc<BackendManager>,
    index: AtomicUsize,
}

impl RoundRobin {
    pub fn new(backend_manager: Arc<BackendManager>) -> Self {
        Self {
            backend_manager,
            index: AtomicUsize::new(0),
        }
    }
}

impl LoadBalancer for RoundRobin {
    fn next_backend(&self) -> Option<Backend> {
        let healthy = self.backend_manager.get_healthy_backends();
        if healthy.is_empty() {
            return None;
        }
        let idx = self.index.fetch_add(1, Ordering::Relaxed) % healthy.len();
        Some(healthy[idx].clone())
    }

    fn name(&self) -> &str {
        "round_robin"
    }
}
