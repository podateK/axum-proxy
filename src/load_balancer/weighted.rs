use std::sync::Arc;

use crate::backend::{Backend, BackendManager};
use crate::load_balancer::LoadBalancer;

pub struct WeightedRoundRobin {
    backend_manager: Arc<BackendManager>,
}

impl WeightedRoundRobin {
    pub fn new(backend_manager: Arc<BackendManager>) -> Self {
        Self { backend_manager }
    }
}

impl LoadBalancer for WeightedRoundRobin {
    fn next_backend(&self) -> Option<Backend> {
        let healthy = self.backend_manager.get_healthy_backends();
        if healthy.is_empty() {
            return None;
        }

        let total_weight: u32 = healthy.iter().map(|b| b.weight).sum();
        if total_weight == 0 {
            return healthy.into_iter().next();
        }

        let mut rand_val = fastrand::u32(0..total_weight);
        for backend in &healthy {
            if rand_val < backend.weight {
                return Some(backend.clone());
            }
            rand_val -= backend.weight;
        }

        healthy.into_iter().last()
    }

    fn name(&self) -> &str {
        "weighted"
    }
}
