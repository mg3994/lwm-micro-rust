use linkwithmentor_common::{DatabaseConfig, RedisConfig, JwtConfig, ServerConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub websocket: WebSocketConfig,
    pub moderation: ModerationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub max_connections_per_user: u32,
    pub heartbeat_interval_seconds: u64,
    pub connection_timeout_seconds: u64,
    pub max_message_size: usize,
    pub rate_limit_messages_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationConfig {
    pub enable_auto_moderation: bool,
    pub safety_service_url: String,
    pub max_message_length: usize,
    pub blocked_words: Vec<String>,
}

impl ChatConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            server: ServerConfig {
                host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "8000".to_string())
                    .parse()
                    .unwrap_or(8000),
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
                    .unwrap_or_else(|_| "0".to_string())
                    .parse()
                    .unwrap_or(0),
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
            websocket: WebSocketConfig {
                max_connections_per_user: std::env::var("WS_MAX_CONNECTIONS_PER_USER")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()
                    .unwrap_or(5),
                heartbeat_interval_seconds: std::env::var("WS_HEARTBEAT_INTERVAL")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
                connection_timeout_seconds: std::env::var("WS_CONNECTION_TIMEOUT")
                    .unwrap_or_else(|_| "300".to_string())
                    .parse()
                    .unwrap_or(300),
                max_message_size: std::env::var("WS_MAX_MESSAGE_SIZE")
                    .unwrap_or_else(|_| "65536".to_string()) // 64KB
                    .parse()
                    .unwrap_or(65536),
                rate_limit_messages_per_minute: std::env::var("WS_RATE_LIMIT_MESSAGES")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()
                    .unwrap_or(60),
            },
            moderation: ModerationConfig {
                enable_auto_moderation: std::env::var("ENABLE_AUTO_MODERATION")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                safety_service_url: std::env::var("SAFETY_SERVICE_URL")
                    .unwrap_or_else(|_| "http://localhost:8006".to_string()),
                max_message_length: std::env::var("MAX_MESSAGE_LENGTH")
                    .unwrap_or_else(|_| "2000".to_string())
                    .parse()
                    .unwrap_or(2000),
                blocked_words: std::env::var("BLOCKED_WORDS")
                    .unwrap_or_else(|_| "".to_string())
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect(),
            },
        })
    }
}