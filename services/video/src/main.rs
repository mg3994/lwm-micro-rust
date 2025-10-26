mod config;
mod handlers;
mod models;
mod signaling;
mod call_manager;
mod webrtc_handler;
mod routes;
mod turn_client;

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

use crate::config::VideoConfig;
use crate::call_manager::CallManager;
use crate::signaling::SignalingService;
use crate::turn_client::TurnClient;

#[derive(Clone)]
pub struct AppState {
    pub config: VideoConfig,
    pub db_pool: sqlx::PgPool,
    pub redis_service: RedisService,
    pub jwt_service: JwtService,
    pub call_manager: CallManager,
    pub signaling_service: SignalingService,
    pub turn_client: TurnClient,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "linkwithmentor_video=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = VideoConfig::from_env()?;

    // Create database connection pool
    let db_pool = create_pool(&config.database).await?;
    
    // Run migrations
    linkwithmentor_database::run_migrations(&db_pool).await?;

    // Create Redis connection
    let redis_service = RedisService::new(&config.redis).await?;

    // Create JWT service
    let jwt_service = JwtService::new(&config.jwt.secret);

    // Create TURN client
    let turn_client = TurnClient::new(&config.turn).await?;

    // Create call manager
    let call_manager = CallManager::new(
        db_pool.clone(),
        redis_service.clone(),
    );

    // Create signaling service
    let signaling_service = SignalingService::new(
        call_manager.clone(),
        turn_client.clone(),
        redis_service.clone(),
    );

    // Initialize signaling service
    signaling_service.initialize().await?;

    // Build application state
    let app_state = AppState {
        config: config.clone(),
        db_pool,
        redis_service,
        jwt_service,
        call_manager,
        signaling_service,
        turn_client,
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

    tracing::info!("Video Service listening on {}:{}", config.server.host, config.server.port);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handler_404() -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("Endpoint not found".to_string())),
    )
}