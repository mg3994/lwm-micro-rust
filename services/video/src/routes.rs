use axum::{
    routing::{get, post, put, delete},
    Router,
    middleware,
};

use linkwithmentor_auth::auth_middleware;

use crate::{
    handlers,
    webrtc_handler::webrtc_handler,
    AppState,
};

pub fn create_routes() -> Router<AppState> {
    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        
        // WebRTC WebSocket endpoint
        .route("/ws", get(webrtc_handler))
        
        // Call management endpoints
        .route("/calls", post(handlers::initiate_call))
        .route("/calls/:call_id/answer", post(handlers::answer_call))
        .route("/calls/:call_id/reject", post(handlers::reject_call))
        .route("/calls/:call_id/end", post(handlers::end_call))
        .route("/calls/:call_id", get(handlers::get_call_info))
        
        // WebRTC signaling endpoints
        .route("/calls/:call_id/ice-candidate", post(handlers::add_ice_candidate))
        .route("/calls/:call_id/media-state", put(handlers::update_media_state))
        
        // Screen sharing endpoints
        .route("/calls/:call_id/screen-share/start", post(handlers::start_screen_share))
        .route("/calls/:call_id/screen-share/stop", post(handlers::stop_screen_share))
        
        // Call quality endpoints
        .route("/calls/:call_id/quality", post(handlers::submit_quality_metrics))
        .route("/calls/:call_id/analytics", get(handlers::get_call_analytics))
        
        // Recording endpoints
        .route("/calls/:call_id/recording/start", post(handlers::start_recording))
        .route("/calls/:call_id/recording/stop", post(handlers::stop_recording))
        
        // ICE servers and TURN credentials
        .route("/ice-servers", get(handlers::get_ice_servers))
        
        // User call management
        .route("/users/calls", get(handlers::get_user_calls))
        
        // Platform statistics (admin only)
        .route("/statistics", get(handlers::get_call_statistics))
        
        // Apply authentication middleware to all routes except health check and WebSocket
        .layer(middleware::from_fn_with_state(
            (), // We'll pass the JWT service through the app state
            auth_middleware,
        ))
}