use axum::{
    routing::{get, post, put, delete},
    Router,
};

use crate::handlers;
use crate::services::AppState;

pub fn create_routes() -> Router<AppState> {
    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        
        // Authentication routes
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/me", get(handlers::get_current_user))
        .route("/auth/switch-role", post(handlers::switch_role))
        
        // Profile management routes
        .route("/profiles/:user_id", get(handlers::get_profile))
        .route("/profiles", put(handlers::update_profile))
        
        // Mentor profile routes
        .route("/mentor-profiles", post(handlers::create_mentor_profile))
        .route("/mentor-profiles", put(handlers::update_mentor_profile))
        .route("/mentor-profiles/:user_id", get(handlers::get_mentor_profile))
        
        // Mentee profile routes
        .route("/mentee-profiles", post(handlers::create_mentee_profile))
        .route("/mentee-profiles", put(handlers::update_mentee_profile))
        .route("/mentee-profiles/:user_id", get(handlers::get_mentee_profile))
        
        // Payment method routes
        .route("/payment-methods", post(handlers::add_payment_method))
        .route("/payment-methods", get(handlers::get_payment_methods))
        .route("/payment-methods/:payment_method_id", put(handlers::update_payment_method))
        .route("/payment-methods/:payment_method_id", delete(handlers::delete_payment_method))
        .route("/payment-methods/:payment_method_id/set-primary", post(handlers::set_primary_payment_method))
        .route("/payment-methods/primary", get(handlers::get_primary_payment_method))
        
        // Role management routes (admin only)
        .route("/users/:user_id/roles", post(handlers::add_role))
        .route("/users/:user_id/roles/:role", delete(handlers::remove_role))
        
        // User management routes
        .route("/users/:user_id", get(handlers::get_user_by_id))
        .route("/users/search", get(handlers::search_users))
}