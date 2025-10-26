use axum::{
    extract::Request,
    http::{StatusCode, HeaderMap},
    Json,
};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use linkwithmentor_common::{ApiResponse, UserRole, RedisKeys};
use linkwithmentor_auth::Claims;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub roles: Vec<UserRole>,
    pub active_role: Option<UserRole>,
    pub session_valid: bool,
}

pub struct AuthService {
    state: AppState,
}

impl AuthService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    // Comprehensive authentication check
    pub async fn authenticate_request(&self, request: &Request) -> Result<AuthContext, (StatusCode, Json<ApiResponse<()>>)> {
        // Extract JWT token
        let token = self.extract_jwt_token(request)?;
        
        // Validate JWT token
        let claims = self.validate_jwt_token(&token)?;
        
        // Check session validity in Redis
        self.validate_session(&claims).await?;
        
        // Check if user is active (not banned/suspended)
        self.check_user_status(&claims).await?;
        
        Ok(AuthContext {
            user_id: claims.sub.clone(),
            username: claims.username.clone(),
            email: claims.email.clone(),
            roles: claims.roles.clone(),
            active_role: claims.active_role.clone(),
            session_valid: true,
        })
    }

    // Extract JWT token from Authorization header
    fn extract_jwt_token(&self, request: &Request) -> Result<String, (StatusCode, Json<ApiResponse<()>>)> {
        let headers = request.headers();
        
        let auth_header = headers
            .get("Authorization")
            .and_then(|header| header.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse::error("Missing Authorization header".to_string())),
                )
            })?;

        if !auth_header.starts_with("Bearer ") {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error("Invalid Authorization header format".to_string())),
            ));
        }

        let token = &auth_header[7..];
        if token.is_empty() {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error("Empty JWT token".to_string())),
            ));
        }

        Ok(token.to_string())
    }

    // Validate JWT token structure and signature
    fn validate_jwt_token(&self, token: &str) -> Result<Claims, (StatusCode, Json<ApiResponse<()>>)> {
        self.state.jwt_service.validate_token(token)
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse::error("Invalid or expired JWT token".to_string())),
                )
            })
    }

    // Validate session exists in Redis
    async fn validate_session(&self, claims: &Claims) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
        let session = self.state.redis_service.get_session(&claims.sub).await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("Session validation failed".to_string())),
                )
            })?;

        if session.is_none() {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error("Session expired or invalid".to_string())),
            ));
        }

        Ok(())
    }

    // Check if user is active (not banned/suspended)
    async fn check_user_status(&self, claims: &Claims) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
        // Check if user is banned or suspended (stored in Redis)
        let ban_key = format!("user_ban:{}", claims.sub);
        let is_banned = self.state.redis_service.cache_get::<bool>(&ban_key).await
            .unwrap_or(Some(false))
            .unwrap_or(false);

        if is_banned {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("User account is suspended".to_string())),
            ));
        }

        Ok(())
    }

    // Role-based authorization
    pub fn authorize_role(&self, auth_context: &AuthContext, required_role: &UserRole) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
        if !auth_context.roles.contains(required_role) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error(format!("Role {:?} required", required_role))),
            ));
        }

        Ok(())
    }

    // Check if user has active role for role-specific operations
    pub fn authorize_active_role(&self, auth_context: &AuthContext, required_role: &UserRole) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
        match &auth_context.active_role {
            Some(active_role) if active_role == required_role => Ok(()),
            Some(active_role) => Err((
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error(format!("Active role must be {:?}, current: {:?}", required_role, active_role))),
            )),
            None => Err((
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error(format!("No active role set, {:?} required", required_role))),
            )),
        }
    }

    // Resource-based authorization (e.g., user can only access their own resources)
    pub fn authorize_resource_access(&self, auth_context: &AuthContext, resource_user_id: &str) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
        // Admin can access any resource
        if auth_context.roles.contains(&UserRole::Admin) {
            return Ok(());
        }

        // User can access their own resources
        if auth_context.user_id == resource_user_id {
            return Ok(());
        }

        Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("Access denied to this resource".to_string())),
        ))
    }

    // Update user presence
    pub async fn update_user_presence(&self, auth_context: &AuthContext) -> Result<(), ()> {
        let role_str = auth_context.active_role.as_ref()
            .map(|r| format!("{:?}", r).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());

        self.state.redis_service.set_user_presence(
            &auth_context.user_id,
            "online",
            &role_str,
        ).await.unwrap_or(());

        Ok(())
    }
}

// Route-based authorization rules
pub struct RouteAuthRules {
    rules: HashMap<String, RouteRule>,
}

#[derive(Debug, Clone)]
pub struct RouteRule {
    pub requires_auth: bool,
    pub required_role: Option<UserRole>,
    pub requires_active_role: bool,
    pub allow_self_access_only: bool,
}

impl RouteAuthRules {
    pub fn new() -> Self {
        let mut rules = HashMap::new();

        // Public routes (no authentication required)
        rules.insert("/health".to_string(), RouteRule {
            requires_auth: false,
            required_role: None,
            requires_active_role: false,
            allow_self_access_only: false,
        });

        rules.insert("/auth/register".to_string(), RouteRule {
            requires_auth: false,
            required_role: None,
            requires_active_role: false,
            allow_self_access_only: false,
        });

        rules.insert("/auth/login".to_string(), RouteRule {
            requires_auth: false,
            required_role: None,
            requires_active_role: false,
            allow_self_access_only: false,
        });

        // Protected routes requiring authentication
        rules.insert("/auth/logout".to_string(), RouteRule {
            requires_auth: true,
            required_role: None,
            requires_active_role: false,
            allow_self_access_only: false,
        });

        rules.insert("/auth/me".to_string(), RouteRule {
            requires_auth: true,
            required_role: None,
            requires_active_role: false,
            allow_self_access_only: true,
        });

        rules.insert("/profiles".to_string(), RouteRule {
            requires_auth: true,
            required_role: None,
            requires_active_role: false,
            allow_self_access_only: true,
        });

        // Mentor-specific routes
        rules.insert("/mentor-profiles".to_string(), RouteRule {
            requires_auth: true,
            required_role: Some(UserRole::Mentor),
            requires_active_role: true,
            allow_self_access_only: true,
        });

        // Mentee-specific routes
        rules.insert("/mentee-profiles".to_string(), RouteRule {
            requires_auth: true,
            required_role: Some(UserRole::Mentee),
            requires_active_role: true,
            allow_self_access_only: true,
        });

        // Payment routes
        rules.insert("/payment-methods".to_string(), RouteRule {
            requires_auth: true,
            required_role: None,
            requires_active_role: false,
            allow_self_access_only: true,
        });

        // Chat routes
        rules.insert("/chat".to_string(), RouteRule {
            requires_auth: true,
            required_role: None,
            requires_active_role: false,
            allow_self_access_only: false,
        });

        // Video call routes
        rules.insert("/video".to_string(), RouteRule {
            requires_auth: true,
            required_role: None,
            requires_active_role: false,
            allow_self_access_only: false,
        });

        // Meeting routes
        rules.insert("/meetings".to_string(), RouteRule {
            requires_auth: true,
            required_role: None,
            requires_active_role: false,
            allow_self_access_only: false,
        });

        // Payment processing routes
        rules.insert("/payments".to_string(), RouteRule {
            requires_auth: true,
            required_role: None,
            requires_active_role: false,
            allow_self_access_only: false,
        });

        // Admin routes
        rules.insert("/users/search".to_string(), RouteRule {
            requires_auth: true,
            required_role: Some(UserRole::Admin),
            requires_active_role: false,
            allow_self_access_only: false,
        });

        Self { rules }
    }

    pub fn get_rule_for_path(&self, path: &str) -> Option<&RouteRule> {
        // Find the most specific matching rule
        let mut best_match = None;
        let mut best_match_len = 0;

        for (rule_path, rule) in &self.rules {
            if path.starts_with(rule_path) && rule_path.len() > best_match_len {
                best_match = Some(rule);
                best_match_len = rule_path.len();
            }
        }

        best_match
    }

    pub fn is_public_route(&self, path: &str) -> bool {
        self.get_rule_for_path(path)
            .map(|rule| !rule.requires_auth)
            .unwrap_or(false)
    }
}

// Helper function to extract user ID from path parameters
pub fn extract_user_id_from_path(path: &str) -> Option<String> {
    // Extract user ID from paths like /users/{user_id} or /profiles/{user_id}
    let segments: Vec<&str> = path.split('/').collect();
    
    for (i, segment) in segments.iter().enumerate() {
        if matches!(*segment, "users" | "profiles" | "mentor-profiles" | "mentee-profiles") {
            if let Some(user_id) = segments.get(i + 1) {
                // Validate that it looks like a UUID
                if user_id.len() == 36 && user_id.chars().filter(|&c| c == '-').count() == 4 {
                    return Some(user_id.to_string());
                }
            }
        }
    }

    None
}