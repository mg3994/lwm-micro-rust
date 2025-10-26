use axum::{
    extract::{Request, State},
    http::{StatusCode, Method},
    response::{Json, Response},
    routing::{any, get},
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};

use linkwithmentor_common::ApiResponse;

use crate::{
    AppState,
    enhanced_proxy,
    health,
    middleware,
    security,
};

pub fn create_routes() -> Router<AppState> {
    Router::new()
        // Health and status endpoints
        .route("/health", get(health::health_check))
        .route("/status", get(health::detailed_status))
        .route("/metrics", get(health::metrics))
        
        // Service discovery endpoints
        .route("/services", get(health::list_services))
        .route("/services/:service_name/health", get(health::service_health))
        
        // Catch-all route for proxying to services
        .fallback(proxy_handler)
        
        // Apply middleware layers
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(middleware::rate_limiting_middleware())
                .layer(middleware::security_headers_middleware())
                .layer(middleware::request_id_middleware())
        )
}

async fn proxy_handler(
    State(state): State<AppState>,
    request: Request,
) -> Result<Response, StatusCode> {
    // Use the enhanced proxy to handle the request
    enhanced_proxy::handle_request(state, request).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// Fallback handler for unmatched routes
pub async fn handler_404() -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("Endpoint not found".to_string())),
    )
}