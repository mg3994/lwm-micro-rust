mod config;
mod handlers;
mod models;
mod dashboards;
mod metrics;
mod reports;
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

use crate::config::AnalyticsConfig;
use crate::dashboards::DashboardService;
use crate::metrics::MetricsService;
use crate::reports::ReportService;

#[derive(Clone)]
pub struct AppState {
    pub config: AnalyticsConfig,
    pub db_pool: sqlx::PgPool,
    pub redis_service: RedisService,
    pub jwt_service: JwtService,
    pub dashboard_service: DashboardService,
    pub metrics_service: MetricsService,
    pub report_service: ReportService,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "linkwithmentor_analytics=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = AnalyticsConfig::from_env()?;

    // Create database connection pool
    let db_pool = create_pool(&config.database).await?;
    
    // Run migrations
    linkwithmentor_database::run_migrations(&db_pool).await?;

    // Create Redis connection
    let redis_service = RedisService::new(&config.redis).await?;

    // Create JWT service
    let jwt_service = JwtService::new(&config.jwt.secret);

    // Create services
    let metrics_service = MetricsService::new(
        db_pool.clone(),
        redis_service.clone(),
    );

    let dashboard_service = DashboardService::new(
        db_pool.clone(),
        metrics_service.clone(),
    );

    let report_service = ReportService::new(
        db_pool.clone(),
        metrics_service.clone(),
    );

    // Initialize services
    metrics_service.initialize().await?;

    // Build application state
    let app_state = AppState {
        config: config.clone(),
        db_pool,
        redis_service,
        jwt_service,
        dashboard_service,
        metrics_service,
        report_service,
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

    tracing::info!("Analytics Service listening on {}:{}", config.server.host, config.server.port);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handler_404() -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("Endpoint not found".to_string())),
    )
}