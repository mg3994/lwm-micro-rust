use axum::{
    extract::Request,
    http::{StatusCode, HeaderMap, HeaderName, HeaderValue},
    Json,
};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

use linkwithmentor_common::ApiResponse;
use crate::AppState;

pub struct SecurityService {
    state: AppState,
}

impl SecurityService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    // Comprehensive security check
    pub async fn security_check(&self, request: &Request) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
        // Check IP-based restrictions
        self.check_ip_restrictions(request).await?;
        
        // Check for suspicious patterns
        self.check_suspicious_patterns(request).await?;
        
        // Validate request headers
        self.validate_headers(request)?;
        
        // Check for common attack patterns
        self.check_attack_patterns(request)?;
        
        // Rate limiting per IP
        self.check_ip_rate_limiting(request).await?;

        Ok(())
    }

    // IP-based restrictions and geolocation
    async fn check_ip_restrictions(&self, request: &Request) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
        let client_ip = self.extract_client_ip(request);
        
        // Check if IP is in blocklist
        let blocklist_key = format!("ip_blocklist:{}", client_ip);
        let is_blocked = self.state.redis_service.cache_get::<bool>(&blocklist_key).await
            .unwrap_or(Some(false))
            .unwrap_or(false);

        if is_blocked {
            tracing::warn!("Blocked IP attempted access: {}", client_ip);
            return Err((
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("Access denied".to_string())),
            ));
        }

        // Check for too many failed attempts from this IP
        let failed_attempts_key = format!("failed_attempts:{}", client_ip);
        let failed_attempts: u32 = self.state.redis_service.cache_get(&failed_attempts_key).await
            .unwrap_or(Some(0))
            .unwrap_or(0);

        if failed_attempts > 10 {
            // Temporarily block IP
            self.state.redis_service.cache_set(&blocklist_key, &true, 3600).await.ok(); // Block for 1 hour
            
            tracing::warn!("IP temporarily blocked due to failed attempts: {}", client_ip);
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(ApiResponse::error("Too many failed attempts. IP temporarily blocked.".to_string())),
            ));
        }

        Ok(())
    }

    // Check for suspicious request patterns
    async fn check_suspicious_patterns(&self, request: &Request) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
        let path = request.uri().path();
        let user_agent = request.headers().get("user-agent")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        // Check for bot patterns
        let bot_patterns = [
            "bot", "crawler", "spider", "scraper", "curl", "wget", "python-requests"
        ];

        if bot_patterns.iter().any(|pattern| user_agent.to_lowercase().contains(pattern)) {
            // Allow legitimate bots but rate limit them more strictly
            let client_ip = self.extract_client_ip(request);
            let bot_rate_key = format!("bot_rate_limit:{}", client_ip);
            
            let allowed = self.state.redis_service.check_rate_limit(&bot_rate_key, 10, 60).await
                .unwrap_or(true);

            if !allowed {
                return Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(ApiResponse::error("Bot rate limit exceeded".to_string())),
                ));
            }
        }

        // Check for path traversal attempts
        if path.contains("..") || path.contains("%2e%2e") {
            tracing::warn!("Path traversal attempt detected: {}", path);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("Invalid path".to_string())),
            ));
        }

        // Check for SQL injection patterns in query parameters
        if let Some(query) = request.uri().query() {
            let sql_patterns = [
                "union", "select", "insert", "delete", "update", "drop", "exec", "script"
            ];
            
            let query_lower = query.to_lowercase();
            if sql_patterns.iter().any(|pattern| query_lower.contains(pattern)) {
                tracing::warn!("Potential SQL injection attempt: {}", query);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error("Invalid query parameters".to_string())),
                ));
            }
        }

        Ok(())
    }

    // Validate request headers
    fn validate_headers(&self, request: &Request) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
        let headers = request.headers();

        // Check for excessively long headers
        for (name, value) in headers.iter() {
            if value.len() > 8192 { // 8KB limit per header
                tracing::warn!("Excessively long header detected: {}", name);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error("Header too long".to_string())),
                ));
            }
        }

        // Check for suspicious header combinations
        let has_x_forwarded_for = headers.contains_key("x-forwarded-for");
        let has_x_real_ip = headers.contains_key("x-real-ip");
        let has_forwarded = headers.contains_key("forwarded");

        // If multiple forwarding headers are present, it might be suspicious
        let forwarding_header_count = [has_x_forwarded_for, has_x_real_ip, has_forwarded]
            .iter()
            .filter(|&&x| x)
            .count();

        if forwarding_header_count > 2 {
            tracing::warn!("Multiple forwarding headers detected - potential header injection");
        }

        Ok(())
    }

    // Check for common attack patterns
    fn check_attack_patterns(&self, request: &Request) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
        let path = request.uri().path();
        
        // Check for common attack paths
        let attack_patterns = [
            "/admin", "/wp-admin", "/phpmyadmin", "/.env", "/config", 
            "/backup", "/test", "/debug", "/.git", "/api/v1/admin"
        ];

        if attack_patterns.iter().any(|pattern| path.starts_with(pattern)) {
            tracing::warn!("Attack pattern detected in path: {}", path);
            
            // Don't block immediately, but log and rate limit more strictly
            let client_ip = self.extract_client_ip(request);
            let attack_key = format!("attack_attempts:{}", client_ip);
            
            // Increment attack counter
            let _ = self.state.redis_service.cache_set(&attack_key, &1, 3600).await;
        }

        Ok(())
    }

    // IP-based rate limiting
    async fn check_ip_rate_limiting(&self, request: &Request) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
        let client_ip = self.extract_client_ip(request);
        let rate_key = format!("ip_rate_limit:{}", client_ip);
        
        // More strict rate limiting per IP (100 requests per minute)
        let allowed = self.state.redis_service.check_rate_limit(&rate_key, 100, 60).await
            .unwrap_or(true);

        if !allowed {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(ApiResponse::error("IP rate limit exceeded".to_string())),
            ));
        }

        Ok(())
    }

    // Extract client IP with proper proxy handling
    fn extract_client_ip(&self, request: &Request) -> String {
        let headers = request.headers();
        
        // Check X-Forwarded-For header (most common)
        if let Some(xff) = headers.get("x-forwarded-for") {
            if let Ok(xff_str) = xff.to_str() {
                // Take the first IP (original client)
                if let Some(first_ip) = xff_str.split(',').next() {
                    let ip = first_ip.trim();
                    if let Ok(_) = IpAddr::from_str(ip) {
                        return ip.to_string();
                    }
                }
            }
        }

        // Check X-Real-IP header
        if let Some(real_ip) = headers.get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                if let Ok(_) = IpAddr::from_str(ip_str) {
                    return ip_str.to_string();
                }
            }
        }

        // Fallback to connection IP (not available in this context, so use placeholder)
        "unknown".to_string()
    }

    // Record failed authentication attempt
    pub async fn record_failed_attempt(&self, client_ip: &str) {
        let failed_attempts_key = format!("failed_attempts:{}", client_ip);
        
        // Increment counter with 1 hour expiry
        let current_count: u32 = self.state.redis_service.cache_get(&failed_attempts_key).await
            .unwrap_or(Some(0))
            .unwrap_or(0);
        
        self.state.redis_service.cache_set(&failed_attempts_key, &(current_count + 1), 3600).await.ok();
    }

    // Clear failed attempts on successful authentication
    pub async fn clear_failed_attempts(&self, client_ip: &str) {
        let failed_attempts_key = format!("failed_attempts:{}", client_ip);
        self.state.redis_service.cache_delete(&failed_attempts_key).await.ok();
    }
}

// CSRF Protection
pub struct CSRFProtection;

impl CSRFProtection {
    pub fn generate_token(user_id: &str, timestamp: i64) -> String {
        let mut hasher = Sha256::new();
        hasher.update(user_id.as_bytes());
        hasher.update(timestamp.to_string().as_bytes());
        hasher.update(b"csrf_secret_key"); // In production, use a proper secret
        
        let result = hasher.finalize();
        general_purpose::STANDARD.encode(result)
    }

    pub fn validate_token(token: &str, user_id: &str, max_age_seconds: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        
        // Try different timestamps within the valid window
        for i in 0..max_age_seconds {
            let timestamp = now - i;
            let expected_token = Self::generate_token(user_id, timestamp);
            
            if token == expected_token {
                return true;
            }
        }
        
        false
    }
}

// Security Headers
pub struct SecurityHeaders;

impl SecurityHeaders {
    pub fn add_security_headers(headers: &mut HeaderMap) {
        // Content Security Policy
        headers.insert(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' wss: https:; font-src 'self' data:; object-src 'none'; media-src 'self'; frame-src 'none';")
        );

        // Strict Transport Security
        headers.insert(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=31536000; includeSubDomains; preload")
        );

        // X-Frame-Options
        headers.insert(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY")
        );

        // X-Content-Type-Options
        headers.insert(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff")
        );

        // X-XSS-Protection
        headers.insert(
            HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block")
        );

        // Referrer Policy
        headers.insert(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin")
        );

        // Permissions Policy
        headers.insert(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=()")
        );

        // Remove server information
        headers.insert(
            HeaderName::from_static("server"),
            HeaderValue::from_static("LinkWithMentor")
        );
    }
}

// DDoS Protection
pub struct DDoSProtection {
    state: AppState,
}

impl DDoSProtection {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    // Detect potential DDoS attacks
    pub async fn check_ddos_patterns(&self, request: &Request) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
        let client_ip = self.extract_client_ip(request);
        
        // Check request frequency
        let freq_key = format!("ddos_freq:{}", client_ip);
        let request_count: u32 = self.state.redis_service.cache_get(&freq_key).await
            .unwrap_or(Some(0))
            .unwrap_or(0);

        // If more than 1000 requests in 1 minute, it's likely DDoS
        if request_count > 1000 {
            tracing::error!("Potential DDoS attack detected from IP: {}", client_ip);
            
            // Block IP for 1 hour
            let block_key = format!("ip_blocklist:{}", client_ip);
            self.state.redis_service.cache_set(&block_key, &true, 3600).await.ok();
            
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(ApiResponse::error("Request rate too high. IP temporarily blocked.".to_string())),
            ));
        }

        // Increment counter
        self.state.redis_service.cache_set(&freq_key, &(request_count + 1), 60).await.ok();

        Ok(())
    }

    fn extract_client_ip(&self, request: &Request) -> String {
        // Same implementation as SecurityService
        let headers = request.headers();
        
        if let Some(xff) = headers.get("x-forwarded-for") {
            if let Ok(xff_str) = xff.to_str() {
                if let Some(first_ip) = xff_str.split(',').next() {
                    let ip = first_ip.trim();
                    if let Ok(_) = IpAddr::from_str(ip) {
                        return ip.to_string();
                    }
                }
            }
        }

        if let Some(real_ip) = headers.get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                if let Ok(_) = IpAddr::from_str(ip_str) {
                    return ip_str.to_string();
                }
            }
        }

        "unknown".to_string()
    }
}