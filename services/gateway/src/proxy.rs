use axum::{
    extract::{Request, State},
    response::{Response, IntoResponse},
    http::{StatusCode, HeaderMap, HeaderName, HeaderValue},
    body::Body,
};
use std::time::Instant;
use std::collections::HashMap;

use linkwithmentor_common::ApiResponse;
use crate::{AppState, config::ServiceConfig};

#[derive(Clone)]
pub struct ProxyService {
    client: reqwest::Client,
    config: crate::config::GatewayConfig,
    load_balancer: crate::load_balancer::LoadBalancer,
}

impl ProxyService {
    pub fn new(config: crate::config::GatewayConfig, load_balancer: crate::load_balancer::LoadBalancer) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            config,
            load_balancer,
        }
    }

    pub async fn proxy_request(&self, service_name: &str, mut request: Request) -> Result<Response<Body>, StatusCode> {
        // Check circuit breaker
        if self.load_balancer.is_circuit_open(service_name).await {
            tracing::warn!("Circuit breaker is open for service: {}", service_name);
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

        // Get service URL
        let service_url = match self.load_balancer.get_service_url(service_name).await {
            Some(url) => url,
            None => {
                tracing::error!("Service {} is not available", service_name);
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        };

        // Record request start
        self.load_balancer.record_request_start(service_name).await;
        let start_time = Instant::now();

        // Build target URL
        let path_and_query = request.uri().path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");
        
        let target_url = format!("{}{}", service_url, path_and_query);

        // Convert Axum request to reqwest request
        let method = request.method().clone();
        let headers = request.headers().clone();
        let body = axum::body::to_bytes(request.into_body(), usize::MAX).await
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        // Build reqwest request
        let mut req_builder = self.client.request(method, &target_url);

        // Copy headers (excluding host and connection headers)
        for (name, value) in headers.iter() {
            let name_str = name.as_str().to_lowercase();
            if !["host", "connection", "content-length"].contains(&name_str.as_str()) {
                req_builder = req_builder.header(name, value);
            }
        }

        // Add body if present
        if !body.is_empty() {
            req_builder = req_builder.body(body);
        }

        // Execute request
        let response_result = req_builder.send().await;
        let response_time = start_time.elapsed().as_millis() as u64;

        match response_result {
            Ok(response) => {
                // Record successful request
                self.load_balancer.record_request_end(service_name, true, response_time).await;

                // Convert reqwest response to Axum response
                self.convert_response(response).await
            }
            Err(err) => {
                // Record failed request
                self.load_balancer.record_request_end(service_name, false, response_time).await;
                
                tracing::error!("Proxy request failed for service {}: {}", service_name, err);
                
                if err.is_timeout() {
                    Err(StatusCode::GATEWAY_TIMEOUT)
                } else if err.is_connect() {
                    Err(StatusCode::BAD_GATEWAY)
                } else {
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
    }

    async fn convert_response(&self, response: reqwest::Response) -> Result<Response<Body>, StatusCode> {
        let status = response.status();
        let headers = response.headers().clone();
        let body_bytes = response.bytes().await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Build Axum response
        let mut response_builder = Response::builder().status(status);

        // Copy headers
        for (name, value) in headers.iter() {
            response_builder = response_builder.header(name, value);
        }

        let response = response_builder
            .body(Body::from(body_bytes))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(response)
    }
}

// Main proxy handler
pub async fn handle_request(
    State(state): State<AppState>,
    request: Request,
) -> impl IntoResponse {
    let path = request.uri().path();
    
    // Determine which service should handle this request
    let service_config = match state.config.get_service_for_path(path) {
        Some(config) => config,
        None => {
            tracing::warn!("No service found for path: {}", path);
            return (
                StatusCode::NOT_FOUND,
                axum::Json(ApiResponse::<()>::error("Service not found".to_string()))
            ).into_response();
        }
    };

    // Apply middleware (authentication, rate limiting, etc.)
    if let Err(response) = apply_middleware(&state, &request).await {
        return response.into_response();
    }

    // Proxy the request
    match state.proxy_service.proxy_request(&service_config.name, request).await {
        Ok(response) => response.into_response(),
        Err(status_code) => {
            let error_message = match status_code {
                StatusCode::SERVICE_UNAVAILABLE => "Service temporarily unavailable",
                StatusCode::GATEWAY_TIMEOUT => "Request timeout",
                StatusCode::BAD_GATEWAY => "Service connection failed",
                _ => "Internal server error",
            };

            (
                status_code,
                axum::Json(ApiResponse::<()>::error(error_message.to_string()))
            ).into_response()
        }
    }
}

async fn apply_middleware(
    state: &AppState,
    request: &Request,
) -> Result<(), (StatusCode, axum::Json<ApiResponse<()>>)> {
    // Apply rate limiting
    if let Err(err) = crate::middleware::rate_limit_middleware(state, request).await {
        return Err(err);
    }

    // Apply authentication for protected routes
    if is_protected_route(request.uri().path()) {
        if let Err(err) = crate::middleware::auth_middleware(state, request).await {
            return Err(err);
        }
    }

    // Apply security headers
    crate::middleware::security_middleware(state, request).await?;

    Ok(())
}

fn is_protected_route(path: &str) -> bool {
    // Define which routes require authentication
    let protected_prefixes = [
        "/auth/logout",
        "/auth/me",
        "/auth/switch-role",
        "/profiles",
        "/mentor-profiles",
        "/mentee-profiles",
        "/payment-methods",
        "/chat",
        "/messages",
        "/video",
        "/calls",
        "/meetings",
        "/sessions",
        "/payments",
        "/transactions",
        "/subscriptions",
    ];

    protected_prefixes.iter().any(|prefix| path.starts_with(prefix))
}

// Helper function to extract service name from path
pub fn extract_service_name(path: &str) -> Option<&'static str> {
    if path.starts_with("/auth") || path.starts_with("/users") || path.starts_with("/profiles") || path.starts_with("/payment-methods") {
        Some("user-management")
    } else if path.starts_with("/chat") || path.starts_with("/messages") {
        Some("chat")
    } else if path.starts_with("/video") || path.starts_with("/calls") {
        Some("video")
    } else if path.starts_with("/meetings") || path.starts_with("/sessions") {
        Some("meetings")
    } else if path.starts_with("/payments") || path.starts_with("/transactions") || path.starts_with("/subscriptions") {
        Some("payment")
    } else if path.starts_with("/safety") || path.starts_with("/moderation") {
        Some("safety")
    } else {
        None
    }
}