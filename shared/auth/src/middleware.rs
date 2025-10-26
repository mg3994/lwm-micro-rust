use axum::{
    extract::{Request, State},
    http::{StatusCode, HeaderMap},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::jwt::{JwtService, Claims};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub email: String,
    pub roles: Vec<String>,
    pub user_type: String,
    pub active_role: Option<String>,
}

impl From<Claims> for AuthenticatedUser {
    fn from(claims: Claims) -> Self {
        Self {
            user_id: claims.user_id,
            email: claims.email,
            roles: claims.roles,
            user_type: claims.user_type,
            active_role: claims.active_role,
        }
    }
}

/// Authentication middleware that validates JWT tokens
pub async fn auth_middleware<S>(
    State(jwt_service): State<JwtService>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract token from Authorization header
    let token = extract_token_from_headers(&headers)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate token
    let claims = jwt_service
        .validate_token(&token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Add claims to request extensions
    request.extensions_mut().insert(claims);

    // Continue to next middleware/handler
    Ok(next.run(request).await)
}

/// Optional authentication middleware that doesn't fail if no token is provided
pub async fn optional_auth_middleware<S>(
    State(jwt_service): State<JwtService>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract and validate token
    if let Some(token) = extract_token_from_headers(&headers) {
        if let Ok(claims) = jwt_service.validate_token(&token) {
            request.extensions_mut().insert(claims);
        }
    }

    // Continue regardless of authentication status
    next.run(request).await
}

/// Role-based authorization middleware
pub fn require_role(required_role: &'static str) -> impl Fn(Request, Next) -> Result<Response, StatusCode> + Clone {
    move |request: Request, next: Next| {
        let claims = request.extensions().get::<Claims>();
        
        match claims {
            Some(claims) if claims.roles.contains(&required_role.to_string()) => {
                Ok(next.run(request).await)
            }
            Some(_) => Err(StatusCode::FORBIDDEN),
            None => Err(StatusCode::UNAUTHORIZED),
        }
    }
}

/// User type authorization middleware
pub fn require_user_type(required_type: &'static str) -> impl Fn(Request, Next) -> Result<Response, StatusCode> + Clone {
    move |request: Request, next: Next| {
        let claims = request.extensions().get::<Claims>();
        
        match claims {
            Some(claims) if claims.user_type == required_type => {
                Ok(next.run(request).await)
            }
            Some(_) => Err(StatusCode::FORBIDDEN),
            None => Err(StatusCode::UNAUTHORIZED),
        }
    }
}

/// Admin-only authorization middleware
pub async fn admin_only_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let claims = request.extensions().get::<Claims>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if claims.roles.contains(&"admin".to_string()) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Mentor-only authorization middleware
pub async fn mentor_only_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let claims = request.extensions().get::<Claims>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if claims.user_type == "mentor" || claims.active_role == Some("mentor".to_string()) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Extract JWT token from Authorization header
fn extract_token_from_headers(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers.get("Authorization")?;
    let auth_str = auth_header.to_str().ok()?;
    
    if auth_str.starts_with("Bearer ") {
        Some(auth_str[7..].to_string())
    } else {
        None
    }
}

/// Extract claims from request extensions (for use in handlers)
pub fn extract_claims(request: &Request) -> Option<&Claims> {
    request.extensions().get::<Claims>()
}

/// Extract authenticated user from request extensions
pub fn extract_user(request: &Request) -> Option<AuthenticatedUser> {
    request.extensions().get::<Claims>().map(|claims| claims.clone().into())
}