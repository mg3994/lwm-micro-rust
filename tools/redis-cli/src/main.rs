use clap::{Parser, Subcommand};
use linkwithmentor_common::{RedisConfig, RedisService};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "redis-cli")]
#[command(about = "LinkWithMentor Redis CLI Tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Test Redis connection
    Test {
        /// Redis URL override
        #[arg(long)]
        redis_url: Option<String>,
    },
    /// Clear all cache data
    FlushCache {
        /// Redis URL override
        #[arg(long)]
        redis_url: Option<String>,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
    /// Show Redis info and statistics
    Info {
        /// Redis URL override
        #[arg(long)]
        redis_url: Option<String>,
    },
    /// Monitor Redis commands in real-time
    Monitor {
        /// Redis URL override
        #[arg(long)]
        redis_url: Option<String>,
    },
    /// Test pub/sub functionality
    TestPubSub {
        /// Redis URL override
        #[arg(long)]
        redis_url: Option<String>,
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
        Commands::Test { redis_url } => {
            let config = get_redis_config(redis_url)?;
            let redis = RedisService::new(&config).await?;
            
            redis.health_check().await?;
            println!("✅ Redis connection successful");
            
            // Test basic operations
            redis.cache_set("test_key", &"test_value", 60).await?;
            let value: Option<String> = redis.cache_get("test_key").await?;
            
            if value == Some("test_value".to_string()) {
                println!("✅ Cache operations working");
            } else {
                println!("❌ Cache operations failed");
            }
            
            redis.cache_delete("test_key").await?;
            println!("✅ All Redis tests passed");
        }
        Commands::FlushCache { redis_url, force } => {
            if !force {
                println!("⚠️  This will delete ALL cached data!");
                println!("Type 'yes' to continue:");
                
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                
                if input.trim() != "yes" {
                    println!("❌ Operation cancelled");
                    return Ok(());
                }
            }
            
            let config = get_redis_config(redis_url)?;
            let redis = RedisService::new(&config).await?;
            let mut conn = redis.get_connection().await?;
            
            redis::cmd("FLUSHDB").execute_async(&mut conn).await?;
            println!("✅ Cache cleared successfully");
        }
        Commands::Info { redis_url } => {
            let config = get_redis_config(redis_url)?;
            let redis = RedisService::new(&config).await?;
            let mut conn = redis.get_connection().await?;
            
            let info: String = redis::cmd("INFO").query_async(&mut conn).await?;
            println!("📊 Redis Information:");
            println!("{}", info);
        }
        Commands::Monitor { redis_url } => {
            let config = get_redis_config(redis_url)?;
            println!("🔍 Monitoring Redis commands (Press Ctrl+C to stop)...");
            
            let client = redis::Client::open(config.connection_string())?;
            let mut conn = client.get_connection()?;
            
            redis::cmd("MONITOR").execute(&mut conn);
        }
        Commands::TestPubSub { redis_url } => {
            let config = get_redis_config(redis_url)?;
            let redis = RedisService::new(&config).await?;
            
            println!("🧪 Testing pub/sub functionality...");
            
            // Test publishing
            redis.publish("test_channel", "Hello, Redis!").await?;
            println!("✅ Published message to test_channel");
            
            // Test JSON publishing
            let test_data = serde_json::json!({
                "message": "Test JSON message",
                "timestamp": chrono::Utc::now().timestamp()
            });
            
            redis.publish_json("test_json_channel", &test_data).await?;
            println!("✅ Published JSON message to test_json_channel");
            
            println!("✅ Pub/sub test completed");
        }
    }

    Ok(())
}

fn get_redis_config(redis_url: Option<String>) -> Result<RedisConfig, Box<dyn std::error::Error>> {
    if let Some(url) = redis_url {
        // Parse Redis URL
        let url = url::Url::parse(&url)?;
        
        Ok(RedisConfig {
            host: url.host_str().unwrap_or("localhost").to_string(),
            port: url.port().unwrap_or(6379),
            password: url.password().map(|p| p.to_string()),
            database: url.path().trim_start_matches('/').parse().unwrap_or(0),
        })
    } else {
        // Load from environment
        Ok(RedisConfig {
            host: std::env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("REDIS_PORT")
                .unwrap_or_else(|_| "6379".to_string())
                .parse()
                .unwrap_or(6379),
            password: std::env::var("REDIS_PASSWORD").ok().filter(|p| !p.is_empty()),
            database: std::env::var("REDIS_DATABASE")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .unwrap_or(0),
        })
    }
}