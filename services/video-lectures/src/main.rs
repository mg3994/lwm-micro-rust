mod config;
mod handlers;
mod models;
mod upload;
mod processing;
mod streaming;
mod storage;
mod analytics;
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

use crate::config::VideoLecturesConfig;
use crate::upload::UploadService;
use crate::processing::VideoProcessingService;
use crate::streaming::StreamingService;
use crate::storage::StorageService;
use crate::analytics::AnalyticsService;

#[derive(Clone)]
pub struct AppState {
    pub config: VideoLecturesConfig,
    pub db_pool: sqlx::PgPool,
    pub redis_service: RedisService,
    pub jwt_service: JwtService,
    pub upload_service: UploadService,
    pub processing_service: VideoProcessingService,
    pub streaming_service: StreamingService,
    pub storage_service: StorageService,
    pub analytics_service: AnalyticsService,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "linkwithmentor_video_lectures=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = VideoLecturesConfig::from_env()?;

    // Create database connection pool
    let db_pool = create_pool(&config.database).await?;
    
    // Run migrations
    linkwithmentor_database::run_migrations(&db_pool).await?;

    // Create Redis connection
    let redis_service = RedisService::new(&config.redis).await?;

    // Create JWT service
    let jwt_service = JwtService::new(&config.jwt.secret);

    // Create storage service
    let storage_service = StorageService::new(&config.storage).await?;

    // Create upload service
    let upload_service = UploadService::new(
        db_pool.clone(),
        storage_service.clone(),
        &config.upload,
    );

    // Create video processing service
    let processing_service = VideoProcessingService::new(
        db_pool.clone(),
        storage_service.clone(),
        redis_service.clone(),
        &config.processing,
    );

    // Create streaming service
    let streaming_service = StreamingService::new(
        storage_service.clone(),
        &config.streaming,
    );

    // Create analytics service
    let analytics_service = AnalyticsService::new(
        db_pool.clone(),
        redis_service.clone(),
    );

    // Initialize services
    processing_service.initialize().await?;

    // Build application state
    let app_state = AppState {
        config: config.clone(),
        db_pool,
        redis_service,
        jwt_service,
        upload_service,
        processing_service,
        streaming_service,
        storage_service,
        analytics_service,
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

    tracing::info!("Video Lectures Service listening on {}:{}", config.server.host, config.server.port);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handler_404() -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("Endpoint not found".to_string())),
    )
}