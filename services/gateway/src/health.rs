use axum::{
    extract::{State, Path},
    response::Json,
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use linkwithmentor_common::ApiResponse;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayHealthResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub services: HashMap<String, ServiceHealthStatus>,
    pub gateway_metrics: GatewayMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceHealthStatus {
    pub name: String,
    pub status: String,
    pub response_time_ms: u64,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub error_count: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayMetrics {
    pub total_requests: u64,
    pub active_connections: u32,
    pub average_response_time_ms: f64,
    pub error_rate_percent: f64,
    pub uptime_seconds: u64,
}

// Gateway health check endpoint
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<GatewayHealthResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Get health status of all services
    let service_health = state.load_balancer.get_all_service_health().await;
    
    let mut services_status = HashMap::new();
    let mut overall_healthy = true;

    for (service_name, health_status) in service_health {
        let status_str = if health_status.is_healthy { "healthy" } else { "unhealthy" };
        
        if !health_status.is_healthy {
            overall_healthy = false;
        }

        services_status.insert(service_name.clone(), ServiceHealthStatus {
            name: service_name,
            status: status_str.to_string(),
            response_time_ms: health_status.response_time_ms,
            last_check: chrono::Utc::now(), // In real implementation, use actual last check time
            error_count: health_status.error_count,
            last_error: health_status.last_error,
        });
    }

    // Calculate gateway metrics
    let gateway_metrics = calculate_gateway_metrics(&state).await;

    let health_response = GatewayHealthResponse {
        status: if overall_healthy { "healthy".to_string() } else { "degraded".to_string() },
        timestamp: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        services: services_status,
        gateway_metrics,
    };

    let status_code = if overall_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok((status_code, Json(ApiResponse::success(health_response))).into())
}

// Individual service health check
pub async fn service_health_check(
    State(state): State<AppState>,
    Path(service_name): Path<String>,
) -> Result<Json<ApiResponse<ServiceHealthStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service_health = state.load_balancer.get_all_service_health().await;
    
    if let Some(health_status) = service_health.get(&service_name) {
        let status_str = if health_status.is_healthy { "healthy" } else { "unhealthy" };
        
        let service_status = ServiceHealthStatus {
            name: service_name,
            status: status_str.to_string(),
            response_time_ms: health_status.response_time_ms,
            last_check: chrono::Utc::now(),
            error_count: health_status.error_count,
            last_error: health_status.last_error.clone(),
        };

        let status_code = if health_status.is_healthy {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };

        Ok((status_code, Json(ApiResponse::success(service_status))).into())
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!("Service '{}' not found", service_name))),
        ))
    }
}

async fn calculate_gateway_metrics(state: &AppState) -> GatewayMetrics {
    // In a real implementation, these metrics would be collected from:
    // - Request counters
    // - Response time histograms
    // - Error rate calculations
    // - Connection pools
    // - System uptime

    // For now, we'll return placeholder values
    // In production, you'd integrate with metrics collection systems like Prometheus

    GatewayMetrics {
        total_requests: 0, // Would be tracked in middleware
        active_connections: 0, // Would be tracked from connection pools
        average_response_time_ms: 0.0, // Would be calculated from response time tracking
        error_rate_percent: 0.0, // Would be calculated from error counters
        uptime_seconds: 0, // Would be calculated from service start time
    }
}

// Readiness probe (for Kubernetes)
pub async fn readiness_check(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Check if critical services are available
    let critical_services = ["user-management"]; // Define which services are critical for readiness
    
    let service_health = state.load_balancer.get_all_service_health().await;
    
    for service_name in critical_services {
        if let Some(health_status) = service_health.get(service_name) {
            if !health_status.is_healthy {
                return Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(ApiResponse::error(format!("Critical service '{}' is not ready", service_name))),
                ));
            }
        } else {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::error(format!("Critical service '{}' not found", service_name))),
            ));
        }
    }

    // Check Redis connectivity
    match state.redis_service.health_check().await {
        Ok(_) => {},
        Err(_) => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::error("Redis is not available".to_string())),
            ));
        }
    }

    Ok(Json(ApiResponse::success("Gateway is ready".to_string())))
}

// Liveness probe (for Kubernetes)
pub async fn liveness_check() -> Json<ApiResponse<String>> {
    // Basic liveness check - just return that the service is alive
    // This should only fail if the service is completely broken
    Json(ApiResponse::success("Gateway is alive".to_string()))
}

// Metrics endpoint (Prometheus format)
pub async fn metrics_endpoint(
    State(state): State<AppState>,
) -> Result<String, (StatusCode, Json<ApiResponse<()>>)> {
    // In a real implementation, this would return Prometheus-formatted metrics
    // For now, we'll return a simple text format
    
    let service_health = state.load_balancer.get_all_service_health().await;
    let mut metrics = String::new();
    
    // Gateway info
    metrics.push_str(&format!("# HELP gateway_info Gateway information\n"));
    metrics.push_str(&format!("# TYPE gateway_info gauge\n"));
    metrics.push_str(&format!("gateway_info{{version=\"{}\"}} 1\n", env!("CARGO_PKG_VERSION")));
    
    // Service health metrics
    metrics.push_str(&format!("# HELP service_health Service health status (1=healthy, 0=unhealthy)\n"));
    metrics.push_str(&format!("# TYPE service_health gauge\n"));
    
    for (service_name, health_status) in service_health {
        let health_value = if health_status.is_healthy { 1 } else { 0 };
        metrics.push_str(&format!("service_health{{service=\"{}\"}} {}\n", service_name, health_value));
        
        // Response time metrics
        metrics.push_str(&format!("service_response_time_ms{{service=\"{}\"}} {}\n", service_name, health_status.response_time_ms));
        
        // Error count metrics
        metrics.push_str(&format!("service_error_count{{service=\"{}\"}} {}\n", service_name, health_status.error_count));
    }

    Ok(metrics)
}