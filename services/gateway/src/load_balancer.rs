use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

use crate::config::{ServiceConfig, LoadBalancerStrategy};

#[derive(Debug, Clone)]
pub struct LoadBalancer {
    services: Arc<RwLock<HashMap<String, ServiceInstance>>>,
    strategy: LoadBalancerStrategy,
}

#[derive(Debug, Clone)]
pub struct ServiceInstance {
    pub config: ServiceConfig,
    pub health_status: HealthStatus,
    pub metrics: ServiceMetrics,
    pub last_health_check: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub response_time_ms: u64,
    pub error_count: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub active_connections: u32,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
}

impl LoadBalancer {
    pub fn new(service_configs: HashMap<String, ServiceConfig>) -> Self {
        let mut services = HashMap::new();
        
        for (name, config) in service_configs {
            services.insert(name.clone(), ServiceInstance {
                config: config.clone(),
                health_status: HealthStatus {
                    is_healthy: true, // Assume healthy initially
                    response_time_ms: 0,
                    error_count: 0,
                    last_error: None,
                },
                metrics: ServiceMetrics {
                    active_connections: 0,
                    total_requests: 0,
                    successful_requests: 0,
                    failed_requests: 0,
                    average_response_time_ms: 0.0,
                },
                last_health_check: Instant::now(),
            });
        }

        let load_balancer = Self {
            services: Arc::new(RwLock::new(services)),
            strategy: LoadBalancerStrategy::RoundRobin, // Default strategy
        };

        // Start health check background task
        load_balancer.start_health_checks();
        
        load_balancer
    }

    pub async fn get_service_url(&self, service_name: &str) -> Option<String> {
        let services = self.services.read().await;
        
        if let Some(instance) = services.get(service_name) {
            if instance.health_status.is_healthy {
                Some(instance.config.base_url.clone())
            } else {
                tracing::warn!("Service {} is unhealthy", service_name);
                None
            }
        } else {
            tracing::error!("Service {} not found", service_name);
            None
        }
    }

    pub async fn record_request_start(&self, service_name: &str) {
        let mut services = self.services.write().await;
        if let Some(instance) = services.get_mut(service_name) {
            instance.metrics.active_connections += 1;
            instance.metrics.total_requests += 1;
        }
    }

    pub async fn record_request_end(&self, service_name: &str, success: bool, response_time_ms: u64) {
        let mut services = self.services.write().await;
        if let Some(instance) = services.get_mut(service_name) {
            instance.metrics.active_connections = instance.metrics.active_connections.saturating_sub(1);
            
            if success {
                instance.metrics.successful_requests += 1;
            } else {
                instance.metrics.failed_requests += 1;
                instance.health_status.error_count += 1;
            }

            // Update average response time
            let total_successful = instance.metrics.successful_requests;
            if total_successful > 0 {
                instance.metrics.average_response_time_ms = 
                    (instance.metrics.average_response_time_ms * (total_successful - 1) as f64 + response_time_ms as f64) / total_successful as f64;
            }
        }
    }

    pub async fn get_service_metrics(&self, service_name: &str) -> Option<ServiceMetrics> {
        let services = self.services.read().await;
        services.get(service_name).map(|instance| instance.metrics.clone())
    }

    pub async fn get_all_service_health(&self) -> HashMap<String, HealthStatus> {
        let services = self.services.read().await;
        services.iter()
            .map(|(name, instance)| (name.clone(), instance.health_status.clone()))
            .collect()
    }

    fn start_health_checks(&self) {
        let services = Arc::clone(&self.services);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                let service_configs = {
                    let services_guard = services.read().await;
                    services_guard.iter()
                        .map(|(name, instance)| (name.clone(), instance.config.clone()))
                        .collect::<Vec<_>>()
                };

                for (service_name, config) in service_configs {
                    let services_clone = Arc::clone(&services);
                    let service_name_clone = service_name.clone();
                    
                    tokio::spawn(async move {
                        let health_result = Self::check_service_health(&config).await;
                        
                        let mut services_guard = services_clone.write().await;
                        if let Some(instance) = services_guard.get_mut(&service_name_clone) {
                            instance.health_status = health_result;
                            instance.last_health_check = Instant::now();
                        }
                    });
                }
            }
        });
    }

    async fn check_service_health(config: &ServiceConfig) -> HealthStatus {
        let client = reqwest::Client::new();
        let health_url = format!("{}{}", config.base_url, config.health_check_path);
        
        let start_time = Instant::now();
        
        match client
            .get(&health_url)
            .timeout(Duration::from_secs(config.timeout_seconds))
            .send()
            .await
        {
            Ok(response) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                
                if response.status().is_success() {
                    HealthStatus {
                        is_healthy: true,
                        response_time_ms: response_time,
                        error_count: 0,
                        last_error: None,
                    }
                } else {
                    let error_msg = format!("Health check failed with status: {}", response.status());
                    tracing::warn!("Service {} health check failed: {}", config.name, error_msg);
                    
                    HealthStatus {
                        is_healthy: false,
                        response_time_ms: response_time,
                        error_count: 1,
                        last_error: Some(error_msg),
                    }
                }
            }
            Err(err) => {
                let error_msg = format!("Health check request failed: {}", err);
                tracing::error!("Service {} health check error: {}", config.name, error_msg);
                
                HealthStatus {
                    is_healthy: false,
                    response_time_ms: 0,
                    error_count: 1,
                    last_error: Some(error_msg),
                }
            }
        }
    }

    // Circuit breaker logic
    pub async fn is_circuit_open(&self, service_name: &str) -> bool {
        let services = self.services.read().await;
        
        if let Some(instance) = services.get(service_name) {
            let failure_threshold = instance.config.circuit_breaker.failure_threshold;
            let timeout_seconds = instance.config.circuit_breaker.timeout_seconds;
            
            // Check if error count exceeds threshold
            if instance.health_status.error_count >= failure_threshold {
                // Check if timeout period has passed
                let timeout_duration = Duration::from_secs(timeout_seconds);
                if instance.last_health_check.elapsed() < timeout_duration {
                    return true; // Circuit is open
                }
            }
        }
        
        false // Circuit is closed
    }

    pub async fn reset_circuit(&self, service_name: &str) {
        let mut services = self.services.write().await;
        if let Some(instance) = services.get_mut(service_name) {
            instance.health_status.error_count = 0;
            instance.health_status.last_error = None;
            tracing::info!("Circuit breaker reset for service: {}", service_name);
        }
    }
}