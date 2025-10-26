use axum::{
    extract::{Request, State},
    http::{StatusCode, HeaderMap},
    middleware::Next,
    response::Response,
    Json,
};
use uuid::Uuid;

use linkwithmentor_common::{ApiResponse, AppError, UserRole};
use linkwithmentor_auth::Claims;

use crate::services::{AppState, UserService};

// Authentication middleware
pub async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<ApiResponse<()>>)> {
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

    let user_service = UserService::new(&state);
    let claims = match user_service.validate_token(token).await {
        Ok(claims) => claims,
        Err(_) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error("Invalid or expired token".to_string())),
            ));
        }
    };

    // Add claims to request extensions
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

// Role-based authorization middleware
pub fn require_role(required_role: UserRole) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, (StatusCode, Json<ApiResponse<()>>)>> + Send>> + Clone {
    move |mut request: Request, next: Next| {
        let required_role = required_role.clone();
        Box::pin(async move {
            let claims = request
                .extensions()
                .get::<Claims>()
                .ok_or_else(|| {
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(ApiResponse::error("Authentication required".to_string())),
                    )
                })?;

            // Check if user has the required role
            if !claims.roles.contains(&required_role) {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::error(format!("Role {:?} required", required_role))),
                ));
            }

            // Check if the required role is the active role (for role-specific operations)
            if let Some(active_role) = &claims.active_role {
                if *active_role != required_role {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(ApiResponse::error(format!("Active role must be {:?}", required_role))),
                    ));
                }
            }

            Ok(next.run(request).await)
        })
    }
}

// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<ApiResponse<()>>)> {
    // Extract user ID from token if available
    let user_id = if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(header_str) = auth_header.to_str() {
            if header_str.starts_with("Bearer ") {
                let token = &header_str[7..];
                if let Ok(user_id) = state.jwt_service.extract_user_id(token) {
                    Some(user_id)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Create rate limit key
    let rate_limit_key = match user_id {
        Some(id) => format!("rate_limit:{}:{}", id, request.uri().path()),
        None => format!("rate_limit:anonymous:{}", 
            headers.get("x-forwarded-for")
                .or_else(|| headers.get("x-real-ip"))
                .and_then(|h| h.to_str().ok())
                .unwrap_or("unknown")
        ),
    };

    // Check rate limit (100 requests per minute for authenticated users, 20 for anonymous)
    let limit = if user_id.is_some() { 100 } else { 20 };
    let window = 60; // 1 minute

    let allowed = state.redis_service
        .check_rate_limit(&rate_limit_key, limit, window)
        .await
        .unwrap_or(true); // Allow on Redis error

    if !allowed {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiResponse::error("Rate limit exceeded".to_string())),
        ));
    }

    Ok(next.run(request).await)
}

// Extract user ID from request
pub fn extract_user_id(request: &Request) -> Result<Uuid, AppError> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| AppError::Authentication("No authentication claims found".to_string()))?;

    Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))
}

// Extract claims from request
pub fn extract_claims(request: &Request) -> Result<&Claims, AppError> {
    request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| AppError::Authentication("No authentication claims found".to_string()))
}