mod config;
mod handlers;
mod models;
mod content_analyzer;
mod moderation_engine;
mod reporting;
mod ml_models;
mod image_analyzer;
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

use crate::config::SafetyConfig;
use crate::content_analyzer::ContentAnalyzer;
use crate::moderation_engine::ModerationEngine;
use crate::reporting::ReportingService;
use crate::ml_models::MLModelManager;

#[derive(Clone)]
pub struct AppState {
    pub config: SafetyConfig,
    pub db_pool: sqlx::PgPool,
    pub redis_service: RedisService,
    pub jwt_service: JwtService,
    pub content_analyzer: ContentAnalyzer,
    pub moderation_engine: ModerationEngine,
    pub reporting_service: ReportingService,
    pub ml_models: MLModelManager,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "linkwithmentor_safety_moderation=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = SafetyConfig::from_env()?;

    // Create database connection pool
    let db_pool = create_pool(&config.database).await?;
    
    // Run migrations
    linkwithmentor_database::run_migrations(&db_pool).await?;

    // Create Redis connection
    let redis_service = RedisService::new(&config.redis).await?;

    // Create JWT service
    let jwt_service = JwtService::new(&config.jwt.secret);

    // Create ML model manager
    let ml_models = MLModelManager::new(&config.ml).await?;

    // Create content analyzer
    let content_analyzer = ContentAnalyzer::new(
        db_pool.clone(),
        redis_service.clone(),
        ml_models.clone(),
    );

    // Create moderation engine
    let moderation_engine = ModerationEngine::new(
        db_pool.clone(),
        redis_service.clone(),
        content_analyzer.clone(),
    );

    // Create reporting service
    let reporting_service = ReportingService::new(
        db_pool.clone(),
        redis_service.clone(),
        moderation_engine.clone(),
    );

    // Initialize services
    content_analyzer.initialize().await?;
    moderation_engine.initialize().await?;
    reporting_service.initialize().await?;

    // Build application state
    let app_state = AppState {
        config: config.clone(),
        db_pool,
        redis_service,
        jwt_service,
        content_analyzer,
        moderation_engine,
        reporting_service,
        ml_models,
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

    tracing::info!("Safety & Moderation Service listening on {}:{}", config.server.host, config.server.port);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handler_404() -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("Endpoint not found".to_string())),
    )
}