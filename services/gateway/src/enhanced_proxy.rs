use axum::{
    extract::{Request, State},
    response::{Response, IntoResponse},
    http::{StatusCode, Method},
    body::Body,
    Json,
};
use std::time::Instant;

use linkwithmentor_common::ApiResponse;
use crate::{AppState, auth::AuthService, middleware::ResponseTimeTracker};

// Enhanced proxy handler with comprehensive middleware
pub async fn handle_enhanced_request(
    State(state): State<AppState>,
    request: Request,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let path = request.uri().path().to_string();
    let method = request.method().clone();
    
    // Start response time tracking
    let response_tracker = ResponseTimeTracker::new();

    // Log incoming request
    crate::middleware::logging_middleware(&request).await;

    // Apply security middleware
    if let Err(response) = crate::middleware::security_middleware(&state, &request).await {
        response_tracker.finish(&path, response.0.as_u16());
        return response.into_response();
    }

    // Apply rate limiting
    if let Err(response) = crate::middleware::rate_limit_middleware(&state, &request).await {
        response_tracker.finish(&path, response.0.as_u16());
        return response.into_response();
    }

    // Find matching route
    let route_config = match state.router.find_route(&path) {
        Some(route) => route,
        None => {
            tracing::warn!("No route found for path: {}", path);
            let response = (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("Service not found".to_string()))
            );
            response_tracker.finish(&path, response.0.as_u16());
            return response.into_response();
        }
    };

    // Check if route requires authentication
    let auth_rule = state.auth_rules.get_rule_for_path(&path);
    let mut auth_context = None;

    if let Some(rule) = auth_rule {
        if rule.requires_auth {
            let auth_service = AuthService::new(state.clone());
            
            match auth_service.authenticate_request(&request).await {
                Ok(context) => {
                    // Check role-based authorization
                    if let Some(required_role) = &rule.required_role {
                        if let Err(response) = auth_service.authorize_role(&context, required_role) {
                            response_tracker.finish(&path, response.0.as_u16());
                            return response.into_response();
                        }

                        // Check active role if required
                        if rule.requires_active_role {
                            if let Err(response) = auth_service.authorize_active_role(&context, required_role) {
                                response_tracker.finish(&path, response.0.as_u16());
                                return response.into_response();
                            }
                        }
                    }

                    // Check resource-based authorization
                    if rule.allow_self_access_only {
                        if let Some(resource_user_id) = crate::auth::extract_user_id_from_path(&path) {
                            if let Err(response) = auth_service.authorize_resource_access(&context, &resource_user_id) {
                                response_tracker.finish(&path, response.0.as_u16());
                                return response.into_response();
                            }
                        }
                    }

                    // Update user presence
                    auth_service.update_user_presence(&context).await.ok();
                    auth_context = Some(context);
                }
                Err(response) => {
                    response_tracker.finish(&path, response.0.as_u16());
                    return response.into_response();
                }
            }
        }
    }

    // Check cache for GET requests
    if method == Method::GET && state.router.should_cache_route(route_config, &method) {
        let cache_key = format!("gateway_cache:{}:{}", method.as_str(), path);
        
        if let Ok(Some(cached_response)) = state.redis_service.cache_get::<Vec<u8>>(&cache_key).await {
            tracing::debug!("Cache hit for: {}", path);
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("X-Cache", "HIT")
                .body(Body::from(cached_response))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .unwrap()
                });
            
            response_tracker.finish(&path, StatusCode::OK.as_u16());
            return response.into_response();
        }
    }

    // Check circuit breaker
    if state.load_balancer.is_circuit_open(&route_config.service_name).await {
        tracing::warn!("Circuit breaker is open for service: {}", route_config.service_name);
        let response = (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<()>::error("Service temporarily unavailable".to_string()))
        );
        response_tracker.finish(&path, response.0.as_u16());
        return response.into_response();
    }

    // Proxy the request with enhanced error handling and retries
    let proxy_result = proxy_with_retries(&state, route_config, request).await;

    match proxy_result {
        Ok(mut response) => {
            let status_code = response.status();
            
            // Add custom headers
            if let Some(context) = &auth_context {
                response.headers_mut().insert("X-User-ID", context.user_id.parse().unwrap());
                if let Some(role) = &context.active_role {
                    response.headers_mut().insert("X-Active-Role", format!("{:?}", role).parse().unwrap());
                }
            }

            // Cache successful GET responses
            if method == Method::GET && status_code.is_success() && state.router.should_cache_route(route_config, &method) {
                if let Some(ttl) = state.router.get_cache_ttl(route_config) {
                    let cache_key = format!("gateway_cache:{}:{}", method.as_str(), path);
                    
                    // Extract response body for caching
                    let (parts, body) = response.into_parts();
                    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
                        Ok(bytes) => bytes,
                        Err(_) => {
                            response_tracker.finish(&path, StatusCode::INTERNAL_SERVER_ERROR.as_u16());
                            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read response body").into_response();
                        }
                    };

                    // Cache the response
                    state.redis_service.cache_set(&cache_key, &body_bytes.to_vec(), ttl).await.ok();

                    // Rebuild response
                    response = Response::from_parts(parts, Body::from(body_bytes));
                    response.headers_mut().insert("X-Cache", "MISS".parse().unwrap());
                }
            }

            response_tracker.finish(&path, status_code.as_u16());
            response.into_response()
        }
        Err(status_code) => {
            let error_message = match status_code {
                StatusCode::SERVICE_UNAVAILABLE => "Service temporarily unavailable",
                StatusCode::GATEWAY_TIMEOUT => "Request timeout",
                StatusCode::BAD_GATEWAY => "Service connection failed",
                _ => "Internal server error",
            };

            let response = (
                status_code,
                Json(ApiResponse::<()>::error(error_message.to_string()))
            );
            
            response_tracker.finish(&path, status_code.as_u16());
            response.into_response()
        }
    }
}

// Proxy with retry logic
async fn proxy_with_retries(
    state: &AppState,
    route_config: &crate::router::RouteConfig,
    request: Request,
) -> Result<Response<Body>, StatusCode> {
    let max_retries = state.router.get_retry_attempts(route_config);
    let mut last_error = StatusCode::INTERNAL_SERVER_ERROR;

    for attempt in 0..=max_retries {
        if attempt > 0 {
            tracing::warn!("Retrying request to {} (attempt {})", route_config.service_name, attempt + 1);
            
            // Exponential backoff
            let delay = std::time::Duration::from_millis(100 * (2_u64.pow(attempt as u32)));
            tokio::time::sleep(delay).await;
        }

        // Clone request for retry (this is simplified - in practice you'd need to handle body cloning)
        let cloned_request = clone_request(&request);
        
        match state.proxy_service.proxy_request(&route_config.service_name, cloned_request).await {
            Ok(response) => {
                // Reset circuit breaker on success
                if response.status().is_success() {
                    state.load_balancer.reset_circuit(&route_config.service_name).await;
                }
                return Ok(response);
            }
            Err(err) => {
                last_error = err;
                
                // Don't retry on client errors (4xx)
                if err.as_u16() >= 400 && err.as_u16() < 500 {
                    break;
                }
                
                // Don't retry payment operations
                if route_config.service_name == "payment" {
                    break;
                }
            }
        }
    }

    Err(last_error)
}

// Helper function to clone request (simplified)
fn clone_request(request: &Request) -> Request {
    // This is a simplified implementation
    // In practice, you'd need to properly handle body cloning
    let mut builder = Request::builder()
        .method(request.method())
        .uri(request.uri());

    // Copy headers
    for (name, value) in request.headers() {
        builder = builder.header(name, value);
    }

    builder.body(Body::empty()).unwrap()
}