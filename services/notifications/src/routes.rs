use axum::{
    routing::{get, post, put},
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
        
        // Notification endpoints
        .route("/notifications", post(handlers::send_notification))
        
        // Preference endpoints
        .route("/preferences", get(handlers::get_preferences))
        .route("/preferences", put(handlers::update_preferences))
        
        // Apply authentication middleware
        .layer(middleware::from_fn_with_state(
            (),
            auth_middleware,
        ))
}