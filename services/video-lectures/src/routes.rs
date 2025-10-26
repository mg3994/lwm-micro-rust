use axum::{
    routing::{get, post, put, delete},
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
        
        // Lecture management endpoints
        .route("/lectures", post(handlers::create_lecture))
        .route("/lectures/search", get(handlers::search_lectures))
        .route("/lectures/:lecture_id", get(handlers::get_lecture))
        .route("/lectures/:lecture_id", put(handlers::update_lecture))
        
        // Upload endpoints
        .route("/lectures/:lecture_id/upload", post(handlers::initiate_upload))
        .route("/uploads/:upload_id/complete/:lecture_id", post(handlers::complete_upload))
        
        // Streaming endpoints
        .route("/lectures/:lecture_id/stream", get(handlers::get_streaming_manifest))
        .route("/lectures/:lecture_id/segments/:segment", get(handlers::get_video_segment))
        
        // Progress tracking
        .route("/lectures/:lecture_id/progress", get(handlers::get_progress))
        .route("/lectures/:lecture_id/progress", post(handlers::update_progress))
        
        // Analytics endpoints
        .route("/lectures/:lecture_id/analytics", get(handlers::get_lecture_analytics))
        .route("/analytics/overview", get(handlers::get_analytics_overview))
        
        // Apply authentication middleware
        .layer(middleware::from_fn_with_state(
            (),
            auth_middleware,
        ))
        
        // Analytics endpoints
        .route("/lectures/:lecture_id/analytics", get(handlers::get_lecture_analytics))
        .route("/lectures/:lecture_id/view", post(handlers::record_view))
        .route("/lectures/:lecture_id/progress", put(handlers::update_watch_progress))
        .route("/lectures/:lecture_id/progress", get(handlers::get_watch_progress))
        
        // Comment endpoints
        .route("/lectures/:lecture_id/comments", post(handlers::create_comment))
        
        // Apply authentication middleware to all routes except health check and public search
        .layer(middleware::from_fn_with_state(
            (), // We'll pass the JWT service through the app state
            auth_middleware,
        ))
}