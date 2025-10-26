use clap::{Parser, Subcommand};
use linkwithmentor_common::{DatabaseConfig, AppConfig};
use linkwithmentor_database::{create_pool, MigrationRunner};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "db-cli")]
#[command(about = "LinkWithMentor Database CLI Tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run database migrations
    Migrate {
        /// Database URL override
        #[arg(long)]
        database_url: Option<String>,
    },
    /// Check migration status
    Status {
        /// Database URL override
        #[arg(long)]
        database_url: Option<String>,
    },
    /// Seed initial data
    Seed {
        /// Database URL override
        #[arg(long)]
        database_url: Option<String>,
    },
    /// Reset database (drop and recreate)
    Reset {
        /// Database URL override
        #[arg(long)]
        database_url: Option<String>,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Migrate { database_url } => {
            let config = get_database_config(database_url)?;
            let pool = create_pool(&config).await?;
            let runner = MigrationRunner::new(pool);
            
            runner.run_all_migrations().await?;
            runner.create_readonly_permissions().await?;
            
            println!("✅ Migrations completed successfully");
        }
        Commands::Status { database_url } => {
            let config = get_database_config(database_url)?;
            let pool = create_pool(&config).await?;
            let runner = MigrationRunner::new(pool);
            
            let status = runner.check_migration_status().await?;
            println!("📊 {}", status);
            
            if status.is_up_to_date {
                println!("✅ Database is up to date");
            } else {
                println!("⚠️  Database needs migration");
            }
        }
        Commands::Seed { database_url } => {
            let config = get_database_config(database_url)?;
            let pool = create_pool(&config).await?;
            let runner = MigrationRunner::new(pool);
            
            runner.seed_initial_data().await?;
            println!("✅ Initial data seeded successfully");
        }
        Commands::Reset { database_url, force } => {
            if !force {
                println!("⚠️  This will delete ALL data in the database!");
                println!("Type 'yes' to continue:");
                
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                
                if input.trim() != "yes" {
                    println!("❌ Operation cancelled");
                    return Ok(());
                }
            }
            
            let config = get_database_config(database_url)?;
            
            // Drop and recreate database
            let admin_config = DatabaseConfig {
                database: "postgres".to_string(),
                ..config.clone()
            };
            
            let admin_pool = create_pool(&admin_config).await?;
            
            // Terminate existing connections
            sqlx::query(&format!(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}' AND pid <> pg_backend_pid()",
                config.database
            ))
            .execute(&admin_pool)
            .await?;
            
            // Drop database
            sqlx::query(&format!("DROP DATABASE IF EXISTS {}", config.database))
                .execute(&admin_pool)
                .await?;
            
            // Create database
            sqlx::query(&format!("CREATE DATABASE {}", config.database))
                .execute(&admin_pool)
                .await?;
            
            // Run migrations on new database
            let pool = create_pool(&config).await?;
            let runner = MigrationRunner::new(pool);
            runner.run_all_migrations().await?;
            runner.create_readonly_permissions().await?;
            
            println!("✅ Database reset completed");
        }
    }

    Ok(())
}

fn get_database_config(database_url: Option<String>) -> Result<DatabaseConfig, Box<dyn std::error::Error>> {
    if let Some(url) = database_url {
        // Parse database URL
        let url = url::Url::parse(&url)?;
        
        Ok(DatabaseConfig {
            host: url.host_str().unwrap_or("localhost").to_string(),
            port: url.port().unwrap_or(5432),
            username: url.username().to_string(),
            password: url.password().unwrap_or("").to_string(),
            database: url.path().trim_start_matches('/').to_string(),
            max_connections: 10,
        })
    } else {
        // Load from environment
        Ok(DatabaseConfig {
            host: std::env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("DATABASE_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .unwrap_or(5432),
            username: std::env::var("DATABASE_USERNAME")
                .unwrap_or_else(|_| "linkwithmentor_user".to_string()),
            password: std::env::var("DATABASE_PASSWORD")
                .unwrap_or_else(|_| "linkwithmentor_password".to_string()),
            database: std::env::var("DATABASE_NAME")
                .unwrap_or_else(|_| "linkwithmentor".to_string()),
            max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
        })
    }
}