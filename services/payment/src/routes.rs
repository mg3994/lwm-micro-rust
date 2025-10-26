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
        
        // Payment endpoints
        .route("/payments", post(handlers::create_payment))
        .route("/payments/:payment_id/refund", post(handlers::refund_payment))
        
        // Subscription endpoints
        .route("/subscriptions", post(handlers::create_subscription))
        
        // Payout endpoints
        .route("/payouts", post(handlers::create_payout))
        
        // Payment method endpoints
        .route("/payment-methods", get(handlers::get_payment_methods))
        .route("/payment-methods", post(handlers::add_payment_method))
        
        // Wallet endpoints
        .route("/wallet", get(handlers::get_wallet))
        
        // Transaction endpoints
        .route("/transactions", get(handlers::get_transactions))
        
        // Analytics endpoints (admin only)
        .route("/analytics", get(handlers::get_payment_analytics))
        
        // Webhook endpoints (no auth required)
        .route("/webhooks/:gateway", post(handlers::handle_webhook))
        
        // Apply authentication middleware to all routes except health check and webhooks
        .layer(middleware::from_fn_with_state(
            (), // We'll pass the JWT service through the app state
            auth_middleware,
        ))
}