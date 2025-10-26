use linkwithmentor_common::{DatabaseConfig, RedisConfig, JwtConfig, ServerConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub analytics: AnalyticsServiceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsServiceConfig {
    pub data_retention_days: u32,
    pub aggregation_interval_minutes: u32,
    pub batch_size: u32,
    pub enable_real_time_metrics: bool,
    pub cache_ttl_seconds: u32,
    pub max_query_results: u32,
}

impl AnalyticsConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            server: ServerConfig {
                host: std::env::var("ANALYTICS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("ANALYTICS_PORT")
                    .unwrap_or_else(|_| "8008".to_string())
                    .parse()
                    .unwrap_or(8008),
                cors_origins: std::env::var("CORS_ORIGINS")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            },
            database: DatabaseConfig {
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
            },
            redis: RedisConfig {
                host: std::env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: std::env::var("REDIS_PORT")
                    .unwrap_or_else(|_| "6379".to_string())
                    .parse()
                    .unwrap_or(6379),
                password: std::env::var("REDIS_PASSWORD").ok().filter(|p| !p.is_empty()),
                database: std::env::var("REDIS_DATABASE")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()
                    .unwrap_or(5),
            },
            jwt: JwtConfig {
                secret: std::env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "dev-secret-key-change-in-production".to_string()),
                expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()
                    .unwrap_or(24),
                issuer: std::env::var("JWT_ISSUER")
                    .unwrap_or_else(|_| "linkwithmentor".to_string()),
            },
            analytics: AnalyticsServiceConfig {
                data_retention_days: std::env::var("ANALYTICS_DATA_RETENTION_DAYS")
                    .unwrap_or_else(|_| "365".to_string())
                    .parse()
                    .unwrap_or(365),
                aggregation_interval_minutes: std::env::var("ANALYTICS_AGGREGATION_INTERVAL")
                    .unwrap_or_else(|_| "15".to_string())
                    .parse()
                    .unwrap_or(15),
                batch_size: std::env::var("ANALYTICS_BATCH_SIZE")
                    .unwrap_or_else(|_| "1000".to_string())
                    .parse()
                    .unwrap_or(1000),
                enable_real_time_metrics: std::env::var("ANALYTICS_REAL_TIME")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                cache_ttl_seconds: std::env::var("ANALYTICS_CACHE_TTL")
                    .unwrap_or_else(|_| "300".to_string())
                    .parse()
                    .unwrap_or(300),
                max_query_results: std::env::var("ANALYTICS_MAX_RESULTS")
                    .unwrap_or_else(|_| "10000".to_string())
                    .parse()
                    .unwrap_or(10000),
            },
        })
    }
}