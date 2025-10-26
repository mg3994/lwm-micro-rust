use axum::{
    extract::Request,
    response::Response,
    http::{StatusCode, HeaderMap, HeaderName, HeaderValue, Method},
    body::Body,
};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use std::sync::Arc;

use crate::AppState;

// Performance monitoring and optimization
pub struct PerformanceMonitor {
    state: AppState,
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub total_response_time_ms: u64,
    pub average_response_time_ms: f64,
    pub requests_per_second: f64,
    pub error_rate: f64,
    pub cache_hit_rate: f64,
    pub service_metrics: HashMap<String, ServicePerformanceMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePerformanceMetrics {
    pub requests: u64,
    pub errors: u64,
    pub total_response_time_ms: u64,
    pub average_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
}

impl PerformanceMonitor {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            metrics: Arc::new(RwLock::new(PerformanceMetrics {
                total_requests: 0,
                total_response_time_ms: 0,
                average_response_time_ms: 0.0,
                requests_per_second: 0.0,
                error_rate: 0.0,
                cache_hit_rate: 0.0,
                service_metrics: HashMap::new(),
            })),
        }
    }

    pub async fn record_request(&self, service_name: &str, response_time_ms: u64, is_error: bool) {
        let mut metrics = self.metrics.write().await;
        
        // Update global metrics
        metrics.total_requests += 1;
        metrics.total_response_time_ms += response_time_ms;
        metrics.average_response_time_ms = metrics.total_response_time_ms as f64 / metrics.total_requests as f64;

        // Update service-specific metrics
        let service_metrics = metrics.service_metrics.entry(service_name.to_string())
            .or_insert(ServicePerformanceMetrics {
                requests: 0,
                errors: 0,
                total_response_time_ms: 0,
                average_response_time_ms: 0.0,
                p95_response_time_ms: 0.0,
                p99_response_time_ms: 0.0,
            });

        service_metrics.requests += 1;
        service_metrics.total_response_time_ms += response_time_ms;
        service_metrics.average_response_time_ms = service_metrics.total_response_time_ms as f64 / service_metrics.requests as f64;

        if is_error {
            service_metrics.errors += 1;
        }

        // Calculate error rate
        metrics.error_rate = (service_metrics.errors as f64 / service_metrics.requests as f64) * 100.0;
    }

    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }

    // Adaptive timeout based on service performance
    pub async fn get_adaptive_timeout(&self, service_name: &str) -> Duration {
        let metrics = self.metrics.read().await;
        
        if let Some(service_metrics) = metrics.service_metrics.get(service_name) {
            // Base timeout on average response time with buffer
            let base_timeout = (service_metrics.average_response_time_ms * 3.0) as u64;
            let timeout_ms = base_timeout.max(5000).min(120000); // Between 5s and 2min
            
            Duration::from_millis(timeout_ms)
        } else {
            Duration::from_secs(30) // Default timeout
        }
    }
}

// Response compression
pub struct CompressionHandler;

impl CompressionHandler {
    pub fn should_compress(headers: &HeaderMap, content_type: Option<&str>) -> bool {
        // Check if client accepts compression
        let accepts_gzip = headers.get("accept-encoding")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.contains("gzip"))
            .unwrap_or(false);

        if !accepts_gzip {
            return false;
        }

        // Check content type
        if let Some(ct) = content_type {
            let compressible_types = [
                "application/json",
                "application/javascript",
                "text/html",
                "text/css",
                "text/plain",
                "text/xml",
                "application/xml",
            ];

            return compressible_types.iter().any(|&t| ct.starts_with(t));
        }

        false
    }

    pub async fn compress_response(response: Response<Body>) -> Response<Body> {
        // This is a placeholder for actual compression implementation
        // In practice, you'd use a compression library like flate2
        
        let (mut parts, body) = response.into_parts();
        
        // Add compression headers
        parts.headers.insert(
            HeaderName::from_static("content-encoding"),
            HeaderValue::from_static("gzip")
        );
        
        parts.headers.insert(
            HeaderName::from_static("vary"),
            HeaderValue::from_static("Accept-Encoding")
        );

        Response::from_parts(parts, body)
    }
}

// Request/Response caching
pub struct CacheManager {
    state: AppState,
}

impl CacheManager {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub async fn get_cached_response(&self, cache_key: &str) -> Option<CachedResponse> {
        self.state.redis_service.cache_get(cache_key).await.unwrap_or(None)
    }

    pub async fn cache_response(&self, cache_key: &str, response: &CachedResponse, ttl: u64) {
        self.state.redis_service.cache_set(cache_key, response, ttl).await.ok();
    }

    pub fn generate_cache_key(&self, method: &Method, path: &str, query: Option<&str>, user_id: Option<&str>) -> String {
        let mut key = format!("gateway_cache:{}:{}", method.as_str(), path);
        
        if let Some(q) = query {
            key.push_str(&format!("?{}", q));
        }
        
        // Include user ID for user-specific caching
        if let Some(uid) = user_id {
            key.push_str(&format!(":user:{}", uid));
        }
        
        key
    }

    pub fn should_cache(&self, method: &Method, path: &str, status_code: StatusCode) -> bool {
        // Only cache GET requests
        if method != Method::GET {
            return false;
        }

        // Only cache successful responses
        if !status_code.is_success() {
            return false;
        }

        // Don't cache certain paths
        let no_cache_paths = [
            "/auth/login",
            "/auth/logout",
            "/payments",
            "/transactions",
        ];

        !no_cache_paths.iter().any(|&p| path.starts_with(p))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub cached_at: i64,
}

// Connection pooling and keep-alive
pub struct ConnectionManager {
    pools: Arc<RwLock<HashMap<String, reqwest::Client>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_client(&self, service_name: &str) -> reqwest::Client {
        let pools = self.pools.read().await;
        
        if let Some(client) = pools.get(service_name) {
            client.clone()
        } else {
            drop(pools);
            
            // Create new client with optimized settings
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .pool_max_idle_per_host(10)
                .pool_idle_timeout(Duration::from_secs(90))
                .tcp_keepalive(Duration::from_secs(60))
                .http2_prior_knowledge()
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            let mut pools = self.pools.write().await;
            pools.insert(service_name.to_string(), client.clone());
            
            client
        }
    }
}

// Request batching for similar requests
pub struct RequestBatcher {
    state: AppState,
    pending_requests: Arc<RwLock<HashMap<String, Vec<BatchedRequest>>>>,
}

#[derive(Debug)]
struct BatchedRequest {
    request_id: String,
    timestamp: Instant,
    // In practice, you'd include the actual request and response channel
}

impl RequestBatcher {
    pub fn new(state: AppState) -> Self {
        let batcher = Self {
            state,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        };

        // Start background task to process batches
        batcher.start_batch_processor();
        
        batcher
    }

    pub async fn should_batch(&self, path: &str) -> bool {
        // Only batch certain types of requests
        let batchable_paths = [
            "/users/search",
            "/mentor-profiles",
            "/mentee-profiles",
        ];

        batchable_paths.iter().any(|&p| path.starts_with(p))
    }

    fn start_batch_processor(&self) {
        let pending_requests = Arc::clone(&self.pending_requests);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            
            loop {
                interval.tick().await;
                
                let mut requests = pending_requests.write().await;
                let now = Instant::now();
                
                // Process batches that are ready (either full or timed out)
                for (batch_key, batch_requests) in requests.iter_mut() {
                    if batch_requests.len() >= 10 || 
                       batch_requests.first().map(|r| now.duration_since(r.timestamp) > Duration::from_millis(50)).unwrap_or(false) {
                        
                        // Process this batch
                        tracing::debug!("Processing batch for {}: {} requests", batch_key, batch_requests.len());
                        
                        // In practice, you'd process the actual requests here
                        batch_requests.clear();
                    }
                }
                
                // Clean up empty batches
                requests.retain(|_, batch| !batch.is_empty());
            }
        });
    }
}

// Performance optimization middleware
pub async fn optimize_response(mut response: Response<Body>) -> Response<Body> {
    let headers = response.headers_mut();
    
    // Add performance headers
    headers.insert(
        HeaderName::from_static("x-response-time"),
        HeaderValue::from_str(&format!("{}ms", 0)).unwrap() // Would be actual response time
    );

    // Add cache control headers for static content
    if let Some(content_type) = headers.get("content-type") {
        if let Ok(ct_str) = content_type.to_str() {
            if ct_str.starts_with("image/") || ct_str.starts_with("text/css") || ct_str.starts_with("application/javascript") {
                headers.insert(
                    HeaderName::from_static("cache-control"),
                    HeaderValue::from_static("public, max-age=31536000") // 1 year
                );
            }
        }
    }

    // Add ETag for caching
    headers.insert(
        HeaderName::from_static("etag"),
        HeaderValue::from_str(&format!("\"{}\"", chrono::Utc::now().timestamp())).unwrap()
    );

    response
}

// Adaptive load balancing based on response times
pub struct AdaptiveLoadBalancer {
    service_response_times: Arc<RwLock<HashMap<String, Vec<u64>>>>,
}

impl AdaptiveLoadBalancer {
    pub fn new() -> Self {
        Self {
            service_response_times: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_response_time(&self, service_instance: &str, response_time_ms: u64) {
        let mut times = self.service_response_times.write().await;
        let instance_times = times.entry(service_instance.to_string()).or_insert_with(Vec::new);
        
        instance_times.push(response_time_ms);
        
        // Keep only last 100 measurements
        if instance_times.len() > 100 {
            instance_times.remove(0);
        }
    }

    pub async fn get_best_instance(&self, service_instances: &[String]) -> Option<String> {
        let times = self.service_response_times.read().await;
        
        let mut best_instance = None;
        let mut best_avg_time = f64::INFINITY;
        
        for instance in service_instances {
            if let Some(instance_times) = times.get(instance) {
                if !instance_times.is_empty() {
                    let avg_time = instance_times.iter().sum::<u64>() as f64 / instance_times.len() as f64;
                    
                    if avg_time < best_avg_time {
                        best_avg_time = avg_time;
                        best_instance = Some(instance.clone());
                    }
                }
            }
        }
        
        best_instance.or_else(|| service_instances.first().cloned())
    }
}