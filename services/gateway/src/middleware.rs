use axum::{
    extract::Request,
    http::{StatusCode, HeaderMap},
    Json,
};
use std::time::SystemTime;

use linkwithmentor_common::{ApiResponse, RedisKeys};
use crate::AppState;

// Rate limiting middleware
pub async fn rate_limit_middleware(
    state: &AppState,
    request: &Request,
) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
    let path = request.uri().path();
    let method = request.method().as_str();
    
    // Extract client identifier (IP or user ID)
    let client_id = extract_client_identifier(request);
    
    // Create rate limit key
    let rate_limit_key = format!("gateway_rate_limit:{}:{}:{}", client_id, method, path);
    
    // Determine rate limits based on authentication status
    let (limit, window) = if is_authenticated_request(request) {
        let base_limit = state.config.rate_limiting.requests_per_minute;
        let multiplier = state.config.rate_limiting.authenticated_multiplier;
        ((base_limit as f32 * multiplier) as u32, 60)
    } else {
        (state.config.rate_limiting.requests_per_minute, 60)
    };

    // Check rate limit
    let allowed = state.redis_service
        .check_rate_limit(&rate_limit_key, limit, window as u64)
        .await
        .unwrap_or(true); // Allow on Redis error

    if !allowed {
        tracing::warn!("Rate limit exceeded for client: {}", client_id);
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiResponse::error("Rate limit exceeded".to_string())),
        ));
    }

    Ok(())
}

// Authentication middleware
pub async fn auth_middleware(
    state: &AppState,
    request: &Request,
) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
    let headers = request.headers();
    
    let auth_header = headers
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| {
            if header.starts_with("Bearer ") {
                Some(&header[7..])
            } else {
                None
            }
        });

    let token = match auth_header {
        Some(token) => token,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error("Missing or invalid authorization header".to_string())),
            ));
        }
    };

    // Validate JWT token
    match state.jwt_service.validate_token(token) {
        Ok(claims) => {
            // Check if session exists in Redis
            let session = state.redis_service.get_session(&claims.sub).await
                .unwrap_or(None);
            
            if session.is_none() {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse::error("Session expired or invalid".to_string())),
                ));
            }

            // Token is valid, continue
            Ok(())
        }
        Err(_) => {
            Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error("Invalid or expired token".to_string())),
            ))
        }
    }
}

// Security middleware
pub async fn security_middleware(
    state: &AppState,
    request: &Request,
) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
    // Check HTTPS requirement
    if state.config.security.require_https {
        let scheme = request.uri().scheme_str().unwrap_or("http");
        if scheme != "https" {
            return Err((
                StatusCode::UPGRADE_REQUIRED,
                Json(ApiResponse::error("HTTPS required".to_string())),
            ));
        }
    }

    // Check request size
    if let Some(content_length) = request.headers().get("content-length") {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                if length > state.config.security.max_request_size {
                    return Err((
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(ApiResponse::error("Request too large".to_string())),
                    ));
                }
            }
        }
    }

    // Check origin for CORS (if not wildcard)
    if !state.config.security.allowed_origins.contains(&"*".to_string()) {
        if let Some(origin) = request.headers().get("origin") {
            if let Ok(origin_str) = origin.to_str() {
                if !state.config.security.allowed_origins.contains(&origin_str.to_string()) {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(ApiResponse::error("Origin not allowed".to_string())),
                    ));
                }
            }
        }
    }

    Ok(())
}

// CSRF protection middleware
pub async fn csrf_middleware(
    state: &AppState,
    request: &Request,
) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
    if !state.config.security.enable_csrf_protection {
        return Ok(());
    }

    let method = request.method();
    
    // Only check CSRF for state-changing methods
    if matches!(method.as_str(), "POST" | "PUT" | "DELETE" | "PATCH") {
        let headers = request.headers();
        
        // Check for CSRF token in header
        let csrf_token = headers.get("X-CSRF-Token")
            .and_then(|header| header.to_str().ok());

        if csrf_token.is_none() {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("CSRF token required".to_string())),
            ));
        }

        // In a real implementation, you would validate the CSRF token
        // For now, we just check that it exists
    }

    Ok(())
}

// Helper functions
fn extract_client_identifier(request: &Request) -> String {
    // Try to extract user ID from JWT token first
    if let Some(auth_header) = request.headers().get("Authorization") {
        if let Ok(header_str) = auth_header.to_str() {
            if header_str.starts_with("Bearer ") {
                let token = &header_str[7..];
                // This is a simplified extraction - in practice you'd parse the JWT
                if let Ok(user_id) = extract_user_id_from_token(token) {
                    return format!("user:{}", user_id);
                }
            }
        }
    }

    // Fall back to IP address
    request.headers()
        .get("x-forwarded-for")
        .or_else(|| request.headers().get("x-real-ip"))
        .and_then(|header| header.to_str().ok())
        .map(|ip| format!("ip:{}", ip))
        .unwrap_or_else(|| "unknown".to_string())
}

fn is_authenticated_request(request: &Request) -> bool {
    request.headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .map(|header| header.starts_with("Bearer "))
        .unwrap_or(false)
}

fn extract_user_id_from_token(token: &str) -> Result<String, ()> {
    // This is a simplified implementation
    // In practice, you would properly decode the JWT
    use base64::{Engine as _, engine::general_purpose};
    
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(());
    }

    // Decode the payload (second part)
    let payload = general_purpose::STANDARD_NO_PAD.decode(parts[1]).map_err(|_| ())?;
    let payload_str = String::from_utf8(payload).map_err(|_| ())?;
    
    // Parse JSON to extract user ID
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload_str) {
        if let Some(sub) = json.get("sub").and_then(|s| s.as_str()) {
            return Ok(sub.to_string());
        }
    }

    Err(())
}

// Request logging middleware
pub async fn logging_middleware(request: &Request) {
    let method = request.method();
    let path = request.uri().path();
    let query = request.uri().query().unwrap_or("");
    let user_agent = request.headers()
        .get("user-agent")
        .and_then(|header| header.to_str().ok())
        .unwrap_or("unknown");

    tracing::info!(
        method = %method,
        path = %path,
        query = %query,
        user_agent = %user_agent,
        "Gateway request"
    );
}

// Response time tracking
pub struct ResponseTimeTracker {
    start_time: SystemTime,
}

impl ResponseTimeTracker {
    pub fn new() -> Self {
        Self {
            start_time: SystemTime::now(),
        }
    }

    pub fn finish(&self, path: &str, status_code: u16) {
        if let Ok(duration) = self.start_time.elapsed() {
            let response_time_ms = duration.as_millis();
            
            tracing::info!(
                path = %path,
                status_code = %status_code,
                response_time_ms = %response_time_ms,
                "Gateway response"
            );

            // Record metrics (could be sent to monitoring system)
            if response_time_ms > 1000 {
                tracing::warn!(
                    path = %path,
                    response_time_ms = %response_time_ms,
                    "Slow response detected"
                );
            }
        }
    }
}