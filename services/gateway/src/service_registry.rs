use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};

use crate::config::ServiceConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    pub service_name: String,
    pub instance_id: String,
    pub base_url: String,
    pub health_status: HealthStatus,
    pub last_health_check: DateTime<Utc>,
    pub response_time_ms: u64,
    pub error_count: u32,
    pub success_count: u32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
    Degraded,
}

pub struct ServiceRegistry {
    services: Arc<RwLock<HashMap<String, Vec<ServiceInstance>>>>,
    health_check_interval: Duration,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            health_check_interval: Duration::seconds(30),
        }
    }

    pub async fn register_service(&self, service_config: &ServiceConfig) {
        let instance = ServiceInstance {
            service_name: service_config.name.clone(),
            instance_id: format!("{}_{}", service_config.name, Utc::now().timestamp()),
            base_url: service_config.base_url.clone(),
            health_status: HealthStatus::Unknown,
            last_health_check: Utc::now(),
            response_time_ms: 0,
            error_count: 0,
            success_count: 0,
            metadata: HashMap::new(),
        };

        let mut services = self.services.write().await;
        services.entry(service_config.name.clone())
            .or_insert_with(Vec::new)
            .push(instance);
    }

    pub async fn get_healthy_instances(&self, service_name: &str) -> Vec<ServiceInstance> {
        let services = self.services.read().await;
        services.get(service_name)
            .map(|instances| {
                instances.iter()
                    .filter(|instance| instance.health_status == HealthStatus::Healthy)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub async fn get_all_instances(&self, service_name: &str) -> Vec<ServiceInstance> {
        let services = self.services.read().await;
        services.get(service_name)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn update_health_status(&self, service_name: &str, instance_id: &str, status: HealthStatus, response_time: u64) {
        let mut services = self.services.write().await;
        if let Some(instances) = services.get_mut(service_name) {
            if let Some(instance) = instances.iter_mut().find(|i| i.instance_id == instance_id) {
                instance.health_status = status.clone();
                instance.last_health_check = Utc::now();
                instance.response_time_ms = response_time;
                
                match status {
                    HealthStatus::Healthy => instance.success_count += 1,
                    HealthStatus::Unhealthy => instance.error_count += 1,
                    _ => {}
                }
            }
        }
    }

    pub async fn record_request_result(&self, service_name: &str, instance_id: &str, success: bool, response_time: u64) {
        let mut services = self.services.write().await;
        if let Some(instances) = services.get_mut(service_name) {
            if let Some(instance) = instances.iter_mut().find(|i| i.instance_id == instance_id) {
                instance.response_time_ms = response_time;
                
                if success {
                    instance.success_count += 1;
                } else {
                    instance.error_count += 1;
                }

                // Update health status based on error rate
                let total_requests = instance.success_count + instance.error_count;
                if total_requests > 10 {
                    let error_rate = instance.error_count as f64 / total_requests as f64;
                    instance.health_status = if error_rate > 0.5 {
                        HealthStatus::Unhealthy
                    } else if error_rate > 0.2 {
                        HealthStatus::Degraded
                    } else {
                        HealthStatus::Healthy
                    };
                }
            }
        }
    }

    pub async fn start_health_checks(&self, service_configs: HashMap<String, ServiceConfig>) {
        let services = self.services.clone();
        let client = reqwest::Client::new();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                let services_read = services.read().await;
                let all_instances: Vec<ServiceInstance> = services_read.values()
                    .flatten()
                    .cloned()
                    .collect();
                drop(services_read);

                for instance in all_instances {
                    if let Some(config) = service_configs.get(&instance.service_name) {
                        let health_url = format!("{}{}", instance.base_url, config.health_check_path);
                        let start_time = std::time::Instant::now();
                        
                        let status = match client.get(&health_url)
                            .timeout(std::time::Duration::from_secs(5))
                            .send()
                            .await
                        {
                            Ok(response) => {
                                if response.status().is_success() {
                                    HealthStatus::Healthy
                                } else {
                                    HealthStatus::Unhealthy
                                }
                            }
                            Err(_) => HealthStatus::Unhealthy,
                        };

                        let response_time = start_time.elapsed().as_millis() as u64;
                        
                        let mut services_write = services.write().await;
                        if let Some(instances) = services_write.get_mut(&instance.service_name) {
                            if let Some(inst) = instances.iter_mut().find(|i| i.instance_id == instance.instance_id) {
                                inst.health_status = status;
                                inst.last_health_check = Utc::now();
                                inst.response_time_ms = response_time;
                            }
                        }
                    }
                }
            }
        });
    }

    pub async fn get_service_stats(&self) -> HashMap<String, ServiceStats> {
        let services = self.services.read().await;
        let mut stats = HashMap::new();

        for (service_name, instances) in services.iter() {
            let healthy_count = instances.iter().filter(|i| i.health_status == HealthStatus::Healthy).count();
            let total_count = instances.len();
            let avg_response_time = if !instances.is_empty() {
                instances.iter().map(|i| i.response_time_ms).sum::<u64>() / instances.len() as u64
            } else {
                0
            };

            stats.insert(service_name.clone(), ServiceStats {
                service_name: service_name.clone(),
                total_instances: total_count,
                healthy_instances: healthy_count,
                average_response_time_ms: avg_response_time,
                uptime_percentage: if total_count > 0 {
                    (healthy_count as f64 / total_count as f64) * 100.0
                } else {
                    0.0
                },
            });
        }

        stats
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStats {
    pub service_name: String,
    pub total_instances: usize,
    pub healthy_instances: usize,
    pub average_response_time_ms: u64,
    pub uptime_percentage: f64,
}