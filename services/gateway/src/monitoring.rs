use axum::{
    extract::{Request, State},
    response::Json,
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use std::sync::Arc;

use linkwithmentor_common::ApiResponse;
use crate::AppState;

// Comprehensive monitoring system
pub struct MonitoringService {
    state: AppState,
    metrics: Arc<RwLock<GatewayMetrics>>,
    alerts: Arc<RwLock<Vec<Alert>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayMetrics {
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub requests_per_second: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub active_connections: u32,
    pub cache_hit_rate: f64,
    pub service_health: HashMap<String, ServiceHealth>,
    pub error_rates: HashMap<String, f64>,
    pub response_time_percentiles: ResponseTimePercentiles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub status: String,
    pub response_time_ms: u64,
    pub error_rate: f64,
    pub last_check: i64,
    pub uptime_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTimePercentiles {
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub level: AlertLevel,
    pub message: String,
    pub service: Option<String>,
    pub timestamp: i64,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

impl MonitoringService {
    pub fn new(state: AppState) -> Self {
        let service = Self {
            state,
            metrics: Arc::new(RwLock::new(GatewayMetrics {
                uptime_seconds: 0,
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                average_response_time_ms: 0.0,
                requests_per_second: 0.0,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
                active_connections: 0,
                cache_hit_rate: 0.0,
                service_health: HashMap::new(),
                error_rates: HashMap::new(),
                response_time_percentiles: ResponseTimePercentiles {
                    p50: 0.0,
                    p90: 0.0,
                    p95: 0.0,
                    p99: 0.0,
                },
            })),
            alerts: Arc::new(RwLock::new(Vec::new())),
        };

        // Start background monitoring tasks
        service.start_monitoring_tasks();
        
        service
    }

    fn start_monitoring_tasks(&self) {
        let metrics = Arc::clone(&self.metrics);
        let alerts = Arc::clone(&self.alerts);
        let state = self.state.clone();

        // System metrics collection
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            let start_time = Instant::now();
            
            loop {
                interval.tick().await;
                
                let mut metrics_guard = metrics.write().await;
                
                // Update uptime
                metrics_guard.uptime_seconds = start_time.elapsed().as_secs();
                
                // Collect system metrics (simplified - in production use proper system monitoring)
                metrics_guard.memory_usage_mb = Self::get_memory_usage().await;
                metrics_guard.cpu_usage_percent = Self::get_cpu_usage().await;
                
                // Update service health
                let service_health = state.load_balancer.get_all_service_health().await;
                for (service_name, health_status) in service_health {
                    metrics_guard.service_health.insert(service_name.clone(), ServiceHealth {
                        status: if health_status.is_healthy { "healthy".to_string() } else { "unhealthy".to_string() },
                        response_time_ms: health_status.response_time_ms,
                        error_rate: 0.0, // Would be calculated from actual metrics
                        last_check: chrono::Utc::now().timestamp(),
                        uptime_percent: 99.9, // Would be calculated from historical data
                    });
                }
                
                drop(metrics_guard);
                
                // Check for alerts
                Self::check_alerts(&metrics, &alerts, &state).await;
            }
        });

        // Request metrics aggregation
        let metrics_clone = Arc::clone(&self.metrics);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Aggregate request metrics from Redis
                // This would collect metrics stored by the gateway during request processing
                let mut metrics_guard = metrics_clone.write().await;
                
                // Calculate requests per second
                if metrics_guard.uptime_seconds > 0 {
                    metrics_guard.requests_per_second = metrics_guard.total_requests as f64 / metrics_guard.uptime_seconds as f64;
                }
                
                // Update cache hit rate
                metrics_guard.cache_hit_rate = Self::calculate_cache_hit_rate().await;
            }
        });
    }

    async fn get_memory_usage() -> f64 {
        // Simplified memory usage - in production use proper system monitoring
        // This would use libraries like sysinfo or procfs
        0.0
    }

    async fn get_cpu_usage() -> f64 {
        // Simplified CPU usage - in production use proper system monitoring
        0.0
    }

    async fn calculate_cache_hit_rate() -> f64 {
        // Calculate cache hit rate from Redis metrics
        // This would track cache hits vs misses
        0.0
    }

    async fn check_alerts(
        metrics: &Arc<RwLock<GatewayMetrics>>,
        alerts: &Arc<RwLock<Vec<Alert>>>,
        state: &AppState,
    ) {
        let metrics_guard = metrics.read().await;
        let mut alerts_guard = alerts.write().await;
        
        // Check for high error rate
        if metrics_guard.failed_requests > 0 {
            let error_rate = (metrics_guard.failed_requests as f64 / metrics_guard.total_requests as f64) * 100.0;
            
            if error_rate > 5.0 { // 5% error rate threshold
                let alert = Alert {
                    id: format!("error_rate_{}", chrono::Utc::now().timestamp()),
                    level: AlertLevel::Warning,
                    message: format!("High error rate detected: {:.2}%", error_rate),
                    service: None,
                    timestamp: chrono::Utc::now().timestamp(),
                    resolved: false,
                };
                
                alerts_guard.push(alert);
            }
        }
        
        // Check for high response times
        if metrics_guard.average_response_time_ms > 1000.0 { // 1 second threshold
            let alert = Alert {
                id: format!("response_time_{}", chrono::Utc::now().timestamp()),
                level: AlertLevel::Warning,
                message: format!("High average response time: {:.2}ms", metrics_guard.average_response_time_ms),
                service: None,
                timestamp: chrono::Utc::now().timestamp(),
                resolved: false,
            };
            
            alerts_guard.push(alert);
        }
        
        // Check service health
        for (service_name, health) in &metrics_guard.service_health {
            if health.status != "healthy" {
                let alert = Alert {
                    id: format!("service_health_{}_{}", service_name, chrono::Utc::now().timestamp()),
                    level: AlertLevel::Error,
                    message: format!("Service {} is unhealthy", service_name),
                    service: Some(service_name.clone()),
                    timestamp: chrono::Utc::now().timestamp(),
                    resolved: false,
                };
                
                alerts_guard.push(alert);
            }
        }
        
        // Clean up old resolved alerts (keep last 100)
        alerts_guard.retain(|alert| {
            let age_hours = (chrono::Utc::now().timestamp() - alert.timestamp) / 3600;
            age_hours < 24 || !alert.resolved
        });
        
        if alerts_guard.len() > 100 {
            alerts_guard.drain(0..alerts_guard.len() - 100);
        }
    }

    pub async fn get_metrics(&self) -> GatewayMetrics {
        self.metrics.read().await.clone()
    }

    pub async fn get_alerts(&self) -> Vec<Alert> {
        self.alerts.read().await.clone()
    }

    pub async fn record_request(&self, service_name: &str, response_time_ms: u64, success: bool) {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_requests += 1;
        
        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }
        
        // Update average response time (simplified moving average)
        let total_response_time = metrics.average_response_time_ms * (metrics.total_requests - 1) as f64 + response_time_ms as f64;
        metrics.average_response_time_ms = total_response_time / metrics.total_requests as f64;
        
        // Update service-specific error rates
        let service_requests = metrics.service_health.get(service_name)
            .map(|h| 1) // Simplified - would track actual request counts
            .unwrap_or(1);
        
        if !success {
            let current_error_rate = metrics.error_rates.get(service_name).unwrap_or(&0.0);
            let new_error_rate = (current_error_rate + 1.0) / service_requests as f64 * 100.0;
            metrics.error_rates.insert(service_name.to_string(), new_error_rate);
        }
    }

    pub async fn record_cache_hit(&self, hit: bool) {
        // Record cache hit/miss for metrics calculation
        let cache_key = if hit { "cache_hits" } else { "cache_misses" };
        
        self.state.redis_service.cache_set(
            &format!("gateway_metrics:{}", cache_key),
            &1,
            3600
        ).await.ok();
    }
}

// Health check endpoints
pub async fn get_gateway_metrics(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<GatewayMetrics>>, (StatusCode, Json<ApiResponse<()>>)> {
    // This would be implemented with actual monitoring service
    let metrics = GatewayMetrics {
        uptime_seconds: 3600, // Placeholder
        total_requests: 1000,
        successful_requests: 950,
        failed_requests: 50,
        average_response_time_ms: 150.0,
        requests_per_second: 10.5,
        memory_usage_mb: 256.0,
        cpu_usage_percent: 15.0,
        active_connections: 25,
        cache_hit_rate: 85.0,
        service_health: HashMap::new(),
        error_rates: HashMap::new(),
        response_time_percentiles: ResponseTimePercentiles {
            p50: 100.0,
            p90: 200.0,
            p95: 300.0,
            p99: 500.0,
        },
    };

    Ok(Json(ApiResponse::success(metrics)))
}

pub async fn get_gateway_alerts(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<Alert>>>, (StatusCode, Json<ApiResponse<()>>)> {
    // This would be implemented with actual monitoring service
    let alerts = vec![
        Alert {
            id: "alert_1".to_string(),
            level: AlertLevel::Warning,
            message: "High response time detected".to_string(),
            service: Some("user-management".to_string()),
            timestamp: chrono::Utc::now().timestamp(),
            resolved: false,
        }
    ];

    Ok(Json(ApiResponse::success(alerts)))
}

// Prometheus metrics endpoint
pub async fn prometheus_metrics(
    State(state): State<AppState>,
) -> Result<String, (StatusCode, Json<ApiResponse<()>>)> {
    let mut metrics = String::new();
    
    // Gateway metrics in Prometheus format
    metrics.push_str("# HELP gateway_requests_total Total number of requests\n");
    metrics.push_str("# TYPE gateway_requests_total counter\n");
    metrics.push_str("gateway_requests_total 1000\n\n");
    
    metrics.push_str("# HELP gateway_request_duration_seconds Request duration in seconds\n");
    metrics.push_str("# TYPE gateway_request_duration_seconds histogram\n");
    metrics.push_str("gateway_request_duration_seconds_bucket{le=\"0.1\"} 100\n");
    metrics.push_str("gateway_request_duration_seconds_bucket{le=\"0.5\"} 800\n");
    metrics.push_str("gateway_request_duration_seconds_bucket{le=\"1.0\"} 950\n");
    metrics.push_str("gateway_request_duration_seconds_bucket{le=\"+Inf\"} 1000\n");
    metrics.push_str("gateway_request_duration_seconds_sum 150.0\n");
    metrics.push_str("gateway_request_duration_seconds_count 1000\n\n");
    
    metrics.push_str("# HELP gateway_active_connections Current active connections\n");
    metrics.push_str("# TYPE gateway_active_connections gauge\n");
    metrics.push_str("gateway_active_connections 25\n\n");
    
    // Service health metrics
    let service_health = state.load_balancer.get_all_service_health().await;
    metrics.push_str("# HELP service_up Service availability\n");
    metrics.push_str("# TYPE service_up gauge\n");
    
    for (service_name, health) in service_health {
        let up_value = if health.is_healthy { 1 } else { 0 };
        metrics.push_str(&format!("service_up{{service=\"{}\"}} {}\n", service_name, up_value));
    }

    Ok(metrics)
}