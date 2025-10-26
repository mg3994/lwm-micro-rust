mod config;
mod handlers;
mod models;
mod scheduling;
mod collaboration;
mod whiteboard;
mod notifications;
mod calendar;
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

use crate::config::MeetingsConfig;
use crate::scheduling::SchedulingService;
use crate::collaboration::CollaborationService;
use crate::whiteboard::WhiteboardService;
use crate::notifications::NotificationService;
use crate::calendar::CalendarService;

#[derive(Clone)]
pub struct AppState {
    pub config: MeetingsConfig,
    pub db_pool: sqlx::PgPool,
    pub redis_service: RedisService,
    pub jwt_service: JwtService,
    pub scheduling_service: SchedulingService,
    pub collaboration_service: CollaborationService,
    pub whiteboard_service: WhiteboardService,
    pub notification_service: NotificationService,
    pub calendar_service: CalendarService,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "linkwithmentor_meetings=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = MeetingsConfig::from_env()?;

    // Create database connection pool
    let db_pool = create_pool(&config.database).await?;
    
    // Run migrations
    linkwithmentor_database::run_migrations(&db_pool).await?;

    // Create Redis connection
    let redis_service = RedisService::new(&config.redis).await?;

    // Create JWT service
    let jwt_service = JwtService::new(&config.jwt.secret);

    // Create notification service
    let notification_service = NotificationService::new(&config.notifications).await?;

    // Create calendar service
    let calendar_service = CalendarService::new(&config.calendar);

    // Create scheduling service
    let scheduling_service = SchedulingService::new(
        db_pool.clone(),
        redis_service.clone(),
        notification_service.clone(),
        calendar_service.clone(),
    );

    // Create whiteboard service
    let whiteboard_service = WhiteboardService::new(
        db_pool.clone(),
        redis_service.clone(),
    );

    // Create collaboration service
    let collaboration_service = CollaborationService::new(
        db_pool.clone(),
        redis_service.clone(),
        whiteboard_service.clone(),
    );

    // Initialize services
    scheduling_service.initialize().await?;
    collaboration_service.initialize().await?;
    whiteboard_service.initialize().await?;

    // Build application state
    let app_state = AppState {
        config: config.clone(),
        db_pool,
        redis_service,
        jwt_service,
        scheduling_service,
        collaboration_service,
        whiteboard_service,
        notification_service,
        calendar_service,
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

    tracing::info!("Meetings Service listening on {}:{}", config.server.host, config.server.port);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handler_404() -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("Endpoint not found".to_string())),
    )
}