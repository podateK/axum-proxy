use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::time;

use crate::backend::BackendManager;
use crate::config::HealthCheckConfig;

pub struct HealthChecker {
    config: HealthCheckConfig,
    backend_manager: Arc<BackendManager>,
    shutdown_tx: watch::Sender<bool>,
}

impl HealthChecker {
    pub fn new(config: HealthCheckConfig, backend_manager: Arc<BackendManager>) -> Self {
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            config,
            backend_manager,
            shutdown_tx,
        }
    }

    pub fn spawn(&self) {
        if !self.config.enabled {
            tracing::info!("Health checks disabled");
            return;
        }

        let interval = Duration::from_secs(self.config.interval_secs);
        let timeout = Duration::from_secs(self.config.timeout_secs);
        let healthy_threshold = self.config.healthy_threshold;
        let unhealthy_threshold = self.config.unhealthy_threshold;
        let manager = self.backend_manager.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            tracing::info!(
                interval_secs = interval.as_secs(),
                "Health checker started"
            );
            let mut ticker = time::interval(interval);
            ticker.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        run_health_checks(
                            &manager,
                            timeout,
                            healthy_threshold,
                            unhealthy_threshold,
                        ).await;
                    }
                    _ = shutdown_rx.changed() => {
                        tracing::info!("Health checker shutting down");
                        break;
                    }
                }
            }
        });
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

async fn run_health_checks(
    manager: &Arc<BackendManager>,
    timeout: Duration,
    healthy_threshold: u32,
    unhealthy_threshold: u32,
) {
    let backends = manager.get_all_backends();

    let mut handles = Vec::with_capacity(backends.len());
    for backend in backends {
        let manager = Arc::clone(manager);
        let url = backend.url.clone();
        let name = backend.name.clone();

        handles.push(tokio::spawn(async move {
            let check_url = format!("{url}/health");
            let result = manager
                .client()
                .get(&check_url)
                .timeout(timeout)
                .send()
                .await;

            let is_healthy = match result {
                Ok(resp) => resp.status().is_success(),
                Err(_) => false,
            };

            if is_healthy {
                if !backend.healthy {
                    let reset_fails = manager.reset_consecutive_failures(&name);
                    if reset_fails == 0 {
                        manager.set_healthy(&name, true);
                        tracing::info!(backend = %name, "Backend recovered");
                    }
                }
            } else {
                let fails = manager.increment_consecutive_failures(&name);
                if fails >= unhealthy_threshold && backend.healthy {
                    manager.set_healthy(&name, false);
                    tracing::warn!(backend = %name, failures = fails, "Backend marked unhealthy");
                }
            }
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }
}
