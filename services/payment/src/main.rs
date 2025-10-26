mod config;
mod handlers;
mod models;
mod gateways;
mod escrow;
mod subscriptions;
mod payouts;
mod encryption;
mod webhooks;
mod routes;

use axum::{
    http::{StatusCode, Method},
    response::Json,
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use linkwithmentor_common::{ApiResponse, RedisService};
use linkwithmentor_database::create_pool;
use linkwithmentor_auth::JwtService;

use crate::config::PaymentConfig;
use crate::gateways::PaymentGatewayManager;
use crate::escrow::EscrowService;
use crate::subscriptions::SubscriptionService;
use crate::payouts::PayoutService;
use crate::encryption::EncryptionService;

#[derive(Clone)]
pub struct AppState {
    pub config: PaymentConfig,
    pub db_pool: sqlx::PgPool,
    pub redis_service: RedisService,
    pub jwt_service: JwtService,
    pub gateway_manager: PaymentGatewayManager,
    pub escrow_service: EscrowService,
    pub subscription_service: SubscriptionService,
    pub payout_service: PayoutService,
    pub encryption_service: EncryptionService,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "linkwithmentor_payment=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = PaymentConfig::from_env()?;

    // Create database connection pool
    let db_pool = create_pool(&config.database).await?;
    
    // Run migrations
    linkwithmentor_database::run_migrations(&db_pool).await?;

    // Create Redis connection
    let redis_service = RedisService::new(&config.redis).await?;

    // Create JWT service
    let jwt_service = JwtService::new(&config.jwt.secret);

    // Create encryption service
    let encryption_service = EncryptionService::new(&config.encryption.key)?;

    // Create payment gateway manager
    let gateway_manager = PaymentGatewayManager::new(&config.gateways).await?;

    // Create escrow service
    let escrow_service = EscrowService::new(
        db_pool.clone(),
        redis_service.clone(),
        gateway_manager.clone(),
    );

    // Create subscription service
    let subscription_service = SubscriptionService::new(
        db_pool.clone(),
        redis_service.clone(),
        gateway_manager.clone(),
    );

    // Create payout service
    let payout_service = PayoutService::new(
        db_pool.clone(),
        redis_service.clone(),
        gateway_manager.clone(),
        encryption_service.clone(),
    );

    // Initialize services
    gateway_manager.initialize().await?;
    subscription_service.initialize().await?;
    payout_service.initialize().await?;

    // Build application state
    let app_state = AppState {
        config: config.clone(),
        db_pool,
        redis_service,
        jwt_service,
        gateway_manager,
        escrow_service,
        subscription_service,
        payout_service,
        encryption_service,
    };

    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any)
        .allow_origin(Any)
        .allow_credentials(true);

    // Build the application
    let app = routes::create_routes()
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
        )
        .with_state(app_state)
        .fallback(handler_404);

    // Start the server
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.server.host, config.server.port))
        .await?;

    tracing::info!("Payment Service listening on {}:{}", config.server.host, config.server.port);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handler_404() -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("Endpoint not found".to_string())),
    )
}