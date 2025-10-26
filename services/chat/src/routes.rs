use axum::{
    routing::{get, post, put, delete},
    Router,
    middleware,
};

use linkwithmentor_auth::auth_middleware;

use crate::{
    handlers,
    websocket::websocket_handler,
    AppState,
};

pub fn create_routes() -> Router<AppState> {
    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        
        // WebSocket endpoint
        .route("/ws", get(websocket_handler))
        
        // Message endpoints
        .route("/messages", post(handlers::send_message))
        .route("/messages/history", get(handlers::get_message_history))
        .route("/messages/:message_id", put(handlers::update_message))
        .route("/messages/:message_id", delete(handlers::delete_message))
        
        // User presence endpoints
        .route("/users/online", get(handlers::get_online_users))
        .route("/rooms/:room_id/participants", get(handlers::get_room_participants))
        
        // Typing indicators
        .route("/typing", post(handlers::send_typing_indicator))
        
        // Group chat endpoints
        .route("/groups", post(handlers::create_group_chat))
        .route("/groups/:group_id/join", post(handlers::join_group_chat))
        .route("/groups/:group_id/leave", post(handlers::leave_group_chat))
        
        // Apply authentication middleware to all routes except health check and WebSocket
        .layer(middleware::from_fn_with_state(
            (), // We'll pass the JWT service through the app state
            auth_middleware,
        ))
}