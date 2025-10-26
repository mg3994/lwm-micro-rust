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
        
        // Analytics overview
        .route("/overview", get(handlers::get_analytics_overview))
        
        // Metrics endpoints
        .route("/metrics/users", get(handlers::get_user_metrics))
        .route("/metrics/sessions", get(handlers::get_session_metrics))
        .route("/metrics/revenue", get(handlers::get_revenue_metrics))
        
        // Event tracking
        .route("/events", post(handlers::track_event))
        
        // Custom queries
        .route("/query", post(handlers::execute_query))
        
        // Dashboard endpoints
        .route("/dashboards", get(handlers::list_dashboards))
        .route("/dashboards", post(handlers::create_dashboard))
        .route("/dashboards/:dashboard_id", get(handlers::get_dashboard))
        .route("/dashboards/:dashboard_id/widgets", post(handlers::add_widget))
        .route("/dashboards/:dashboard_id/widgets/:widget_id", delete(handlers::remove_widget))
        
        // Report endpoints
        .route("/reports", get(handlers::list_reports))
        .route("/reports", post(handlers::create_report))
        .route("/reports/:report_id", get(handlers::get_report))
        .route("/reports/:report_id", delete(handlers::delete_report))
        .route("/reports/:report_id/generate", post(handlers::generate_report))
        
        // Apply authentication middleware
        .layer(middleware::from_fn_with_state(
            (),
            auth_middleware,
        ))
}