use sqlx::PgPool;
use linkwithmentor_common::AppError;

pub struct MigrationRunner {
    pool: PgPool,
}

impl MigrationRunner {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn run_all_migrations(&self) -> Result<(), AppError> {
        tracing::info!("Starting database migrations...");
        
        let migrator = sqlx::migrate!("./migrations");
        migrator.run(&self.pool)
            .await
            .map_err(AppError::Database)?;
        
        tracing::info!("All migrations completed successfully");
        Ok(())
    }

    pub async fn check_migration_status(&self) -> Result<MigrationStatus, AppError> {
        let migrator = sqlx::migrate!("./migrations");
        let applied = migrator.get_applied_migrations(&self.pool)
            .await
            .map_err(AppError::Database)?;
        
        let total_migrations = migrator.migrations.len();
        let applied_count = applied.len();
        let pending_count = total_migrations - applied_count;

        Ok(MigrationStatus {
            total: total_migrations,
            applied: applied_count,
            pending: pending_count,
            is_up_to_date: pending_count == 0,
        })
    }

    pub async fn create_readonly_permissions(&self) -> Result<(), AppError> {
        // Grant read-only permissions to analytics user
        let queries = vec![
            "GRANT USAGE ON SCHEMA public TO linkwithmentor_readonly;",
            "GRANT SELECT ON ALL TABLES IN SCHEMA public TO linkwithmentor_readonly;",
            "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO linkwithmentor_readonly;",
        ];

        for query in queries {
            sqlx::query(query)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;
        }

        tracing::info!("Read-only permissions granted");
        Ok(())
    }

    pub async fn seed_initial_data(&self) -> Result<(), AppError> {
        // Create admin user if it doesn't exist
        let admin_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)"
        )
        .bind("admin@linkwithmentor.com")
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if !admin_exists {
            let admin_password = linkwithmentor_auth::PasswordService::hash_password("admin123!")?;
            
            sqlx::query(
                r#"
                INSERT INTO users (username, email, roles, hashed_password, email_verified)
                VALUES ($1, $2, $3, $4, $5)
                "#
            )
            .bind("admin")
            .bind("admin@linkwithmentor.com")
            .bind(vec!["admin"])
            .bind(admin_password)
            .bind(true)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

            tracing::info!("Admin user created");
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct MigrationStatus {
    pub total: usize,
    pub applied: usize,
    pub pending: usize,
    pub is_up_to_date: bool,
}

impl std::fmt::Display for MigrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Migrations: {}/{} applied, {} pending",
            self.applied, self.total, self.pending
        )
    }
}