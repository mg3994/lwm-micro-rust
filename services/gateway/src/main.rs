mod config;
mod proxy;
mod middleware;
mod health;
mod load_balancer;
mod auth;
mod router;
mod enhanced_proxy;
mod security;
mod performance;
mod monitoring;
mod service_registry;

use axum::{
    http::{StatusCode, Method},
    response::Json,
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    timeout::TimeoutLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::time::Duration;

use linkwithmentor_common::{ApiResponse, RedisService};
use linkwithmentor_auth::JwtService;

use crate::config::GatewayConfig;
use crate::proxy::ProxyService;
use crate::load_balancer::LoadBalancer;
use crate::service_registry::ServiceRegistry;

#[derive(Clone)]
pub struct AppState {
    pub config: GatewayConfig,
    pub redis_service: RedisService,
    pub jwt_service: JwtService,
    pub proxy_service: ProxyService,
    pub load_balancer: LoadBalancer,
    pub router: router::Router,
    pub auth_rules: auth::RouteAuthRules,
    pub service_registry: ServiceRegistry,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "linkwithmentor_gateway=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = GatewayConfig::from_env()?;

    // Create Redis connection
    let redis_service = RedisService::new(&config.redis).await?;

    // Create JWT service
    let jwt_service = JwtService::new(&config.jwt.secret);

    // Create load balancer
    let load_balancer = LoadBalancer::new(config.services.clone());

    // Create router
    let router = router::Router::new(config.services.clone());

    // Create auth rules
    let auth_rules = auth::RouteAuthRules::new();

    // Create proxy service
    let proxy_service = ProxyService::new(config.clone(), load_balancer.clone());

    // Build application state
    let app_state = AppState {
        config: config.clone(),
        redis_service,
        jwt_service,
        proxy_service,
        load_balancer,
        router,
        auth_rules,
    };

    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH, Method::OPTIONS])
        .allow_headers(Any)
        .allow_origin(Any)
        .allow_credentials(true);

    // Build the application
    let app = Router::new()
        // Health check endpoints
        .route("/health", axum::routing::get(health::health_check))
        .route("/health/:service", axum::routing::get(health::service_health_check))
        .route("/ready", axum::routing::get(health::readiness_check))
        .route("/live", axum::routing::get(health::liveness_check))
        .route("/metrics", axum::routing::get(health::metrics_endpoint))
        
        // Monitoring endpoints
        .route("/admin/metrics", axum::routing::get(monitoring::get_gateway_metrics))
        .route("/admin/alerts", axum::routing::get(monitoring::get_gateway_alerts))
        .route("/admin/prometheus", axum::routing::get(monitoring::prometheus_metrics))
        
        // Proxy all other requests to appropriate services
        .fallback(enhanced_proxy::handle_enhanced_request)
        
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(cors)
        )
        .with_state(app_state);

    // Start the server
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.server.host, config.server.port))
        .await?;

    tracing::info!("Gateway Service listening on {}:{}", config.server.host, config.server.port);
    tracing::info!("Configured services: {:?}", config.services.keys().collect::<Vec<_>>());

    axum::serve(listener, app).await?;

    Ok(())
}