mod config;
mod handlers;
mod models;
mod email;
mod sms;
mod push;
mod templates;
mod scheduler;
mod delivery;
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

use crate::config::NotificationConfig;
use crate::email::EmailService;
use crate::sms::SmsService;
use crate::push::PushService;
use crate::templates::TemplateEngine;
use crate::scheduler::NotificationScheduler;
use crate::delivery::DeliveryManager;

#[derive(Clone)]
pub struct AppState {
    pub config: NotificationConfig,
    pub db_pool: sqlx::PgPool,
    pub redis_service: RedisService,
    pub jwt_service: JwtService,
    pub email_service: EmailService,
    pub sms_service: SmsService,
    pub push_service: PushService,
    pub template_engine: TemplateEngine,
    pub scheduler: NotificationScheduler,
    pub delivery_manager: DeliveryManager,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "linkwithmentor_notifications=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = NotificationConfig::from_env()?;

    // Create database connection pool
    let db_pool = create_pool(&config.database).await?;
    
    // Run migrations
    linkwithmentor_database::run_migrations(&db_pool).await?;

    // Create Redis connection
    let redis_service = RedisService::new(&config.redis).await?;

    // Create JWT service
    let jwt_service = JwtService::new(&config.jwt.secret);

    // Create notification services
    let email_service = EmailService::new(&config.email).await?;
    let sms_service = SmsService::new(&config.sms).await?;
    let push_service = PushService::new(&config.push).await?;
    
    // Create template engine
    let template_engine = TemplateEngine::new(&config.templates)?;
    
    // Create scheduler
    let scheduler = NotificationScheduler::new(
        db_pool.clone(),
        redis_service.clone(),
    ).await?;
    
    // Create delivery manager
    let delivery_manager = DeliveryManager::new(
        db_pool.clone(),
        redis_service.clone(),
        email_service.clone(),
        sms_service.clone(),
        push_service.clone(),
        template_engine.clone(),
    );

    // Initialize services
    scheduler.start().await?;
    delivery_manager.start_workers().await?;

    // Build application state
    let app_state = AppState {
        config: config.clone(),
        db_pool,
        redis_service,
        jwt_service,
        email_service,
        sms_service,
        push_service,
        template_engine,
        scheduler,
        delivery_manager,
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

    tracing::info!("Notification Service listening on {}:{}", config.server.host, config.server.port);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handler_404() -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("Endpoint not found".to_string())),
    )
}