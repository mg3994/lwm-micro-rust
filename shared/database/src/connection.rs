use sqlx::{PgPool, Pool, Postgres, migrate::MigrateDatabase};
use linkwithmentor_common::{DatabaseConfig, AppError};

pub type DbPool = Pool<Postgres>;

pub async fn create_pool(config: &DatabaseConfig) -> Result<DbPool, AppError> {
    let connection_string = config.connection_string();
    
    // Create database if it doesn't exist
    if !Postgres::database_exists(&connection_string).await.unwrap_or(false) {
        tracing::info!("Creating database: {}", config.database);
        Postgres::create_database(&connection_string)
            .await
            .map_err(AppError::Database)?;
    }
    
    let pool = PgPool::connect(&connection_string)
        .await
        .map_err(AppError::Database)?;
    
    // Test the connection
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(AppError::Database)?;
    
    tracing::info!("Database connection established");
    Ok(pool)
}

pub async fn run_migrations(pool: &DbPool) -> Result<(), AppError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(AppError::Database)?;
    
    tracing::info!("Database migrations completed");
    Ok(())
}

// Helper function to check if migrations are needed
pub async fn check_migration_status(pool: &DbPool) -> Result<bool, AppError> {
    let migrator = sqlx::migrate!("./migrations");
    let applied = migrator.get_applied_migrations(pool)
        .await
        .map_err(AppError::Database)?;
    
    let pending = migrator.migrations.len() - applied.len();
    
    if pending > 0 {
        tracing::info!("Found {} pending migrations", pending);
        Ok(true)
    } else {
        tracing::info!("All migrations are up to date");
        Ok(false)
    }
}