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
        
        // Session management endpoints
        .route("/sessions", post(handlers::create_session))
        .route("/sessions", get(handlers::list_sessions))
        .route("/sessions/:session_id", get(handlers::get_session))
        .route("/sessions/:session_id", put(handlers::update_session))
        .route("/sessions/:session_id", delete(handlers::cancel_session))
        
        // Session booking and scheduling
        .route("/sessions/:session_id/book", post(handlers::book_session))
        .route("/sessions/:session_id/confirm", post(handlers::confirm_session))
        .route("/sessions/:session_id/reschedule", put(handlers::reschedule_session))
        
        // Availability management
        .route("/availability", get(handlers::get_availability))
        .route("/availability", post(handlers::set_availability))
        .route("/availability/:availability_id", put(handlers::update_availability))
        .route("/availability/:availability_id", delete(handlers::delete_availability))
        
        // Calendar integration
        .route("/calendar/events", get(handlers::get_calendar_events))
        .route("/calendar/sync", post(handlers::sync_calendar))
        
        // Whiteboard endpoints
        .route("/sessions/:session_id/whiteboard", get(handlers::get_whiteboard_state))
        .route("/sessions/:session_id/whiteboard", post(handlers::update_whiteboard_state))
        .route("/sessions/:session_id/whiteboard/clear", post(handlers::clear_whiteboard))
        
        // Materials and file sharing
        .route("/sessions/:session_id/materials", get(handlers::list_materials))
        .route("/sessions/:session_id/materials", post(handlers::upload_material))
        .route("/sessions/:session_id/materials/:material_id", get(handlers::get_material))
        .route("/sessions/:session_id/materials/:material_id", delete(handlers::delete_material))
        
        // Session notes
        .route("/sessions/:session_id/notes", get(handlers::get_session_notes))
        .route("/sessions/:session_id/notes", post(handlers::create_session_notes))
        .route("/sessions/:session_id/notes", put(handlers::update_session_notes))
        
        // Recurring sessions
        .route("/recurring-sessions", post(handlers::create_recurring_session))
        .route("/recurring-sessions", get(handlers::list_recurring_sessions))
        .route("/recurring-sessions/:recurring_id", get(handlers::get_recurring_session))
        .route("/recurring-sessions/:recurring_id", put(handlers::update_recurring_session))
        .route("/recurring-sessions/:recurring_id", delete(handlers::delete_recurring_session))
        
        // Session analytics
        .route("/sessions/:session_id/analytics", get(handlers::get_session_analytics))
        .route("/analytics/sessions", get(handlers::get_sessions_analytics))
        
        // Apply authentication middleware
        .layer(middleware::from_fn_with_state(
            (),
            auth_middleware,
        ))
}