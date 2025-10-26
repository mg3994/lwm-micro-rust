use axum::{
    routing::{get, post},
    Router,
    middleware,
};

use linkwithmentor_auth::auth_middleware;

use crate::{
    handlers,
    AppState,
};

pub fn create_routes() -> Router<AppState> {
    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        
        // Content analysis endpoints
        .route("/analyze/content", post(handlers::analyze_content))
        .route("/analyze/image", post(handlers::analyze_image))
        
        // Moderation action endpoints
        .route("/moderation/actions", post(handlers::execute_moderation_action))
        
        // Reporting endpoints
        .route("/reports", post(handlers::submit_report))
        
        // User safety endpoints
        .route("/users/:user_id/safety-profile", get(handlers::get_user_safety_profile))
        
        // Analytics endpoints
        .route("/analytics", get(handlers::get_moderation_analytics))
        
        // Apply authentication middleware to all routes except health check
        .layer(middleware::from_fn_with_state(
            (), // We'll pass the JWT service through the app state
            auth_middleware,
        ))
}