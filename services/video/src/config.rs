use linkwithmentor_common::{DatabaseConfig, RedisConfig, JwtConfig, ServerConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub video: VideoServiceConfig,
    pub turn: TurnConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoServiceConfig {
    pub max_participants_per_call: u32,
    pub call_timeout_seconds: u64,
    pub recording_enabled: bool,
    pub recording_storage_path: String,
    pub max_call_duration_minutes: u32,
    pub enable_screen_sharing: bool,
    pub enable_call_recording: bool,
    pub video_quality_levels: Vec<String>,
    pub audio_codecs: Vec<String>,
    pub video_codecs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnConfig {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub realm: String,
    pub static_auth_secret: Option<String>,
    pub ttl_seconds: u32,
}

impl VideoConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            server: ServerConfig {
                host: std::env::var("VIDEO_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("VIDEO_PORT")
                    .unwrap_or_else(|_| "8003".to_string())
                    .parse()
                    .unwrap_or(8003),
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
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()
                    .unwrap_or(1),
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
            video: VideoServiceConfig {
                max_participants_per_call: std::env::var("VIDEO_MAX_PARTICIPANTS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
                call_timeout_seconds: std::env::var("VIDEO_CALL_TIMEOUT")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse()
                    .unwrap_or(3600),
                recording_enabled: std::env::var("VIDEO_RECORDING_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                recording_storage_path: std::env::var("VIDEO_RECORDING_PATH")
                    .unwrap_or_else(|_| "/app/recordings".to_string()),
                max_call_duration_minutes: std::env::var("VIDEO_MAX_DURATION")
                    .unwrap_or_else(|_| "120".to_string())
                    .parse()
                    .unwrap_or(120),
                enable_screen_sharing: std::env::var("VIDEO_ENABLE_SCREEN_SHARING")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                enable_call_recording: std::env::var("VIDEO_ENABLE_CALL_RECORDING")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                video_quality_levels: std::env::var("VIDEO_QUALITY_LEVELS")
                    .unwrap_or_else(|_| "low,medium,high".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                audio_codecs: std::env::var("VIDEO_AUDIO_CODECS")
                    .unwrap_or_else(|_| "opus,pcmu,pcma".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                video_codecs: std::env::var("VIDEO_VIDEO_CODECS")
                    .unwrap_or_else(|_| "vp8,vp9,h264".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            },
            turn: TurnConfig {
                server_url: std::env::var("TURN_SERVER_URL")
                    .unwrap_or_else(|_| "turn:localhost:3478".to_string()),
                username: std::env::var("TURN_USERNAME")
                    .unwrap_or_else(|_| "linkwithmentor".to_string()),
                password: std::env::var("TURN_PASSWORD")
                    .unwrap_or_else(|_| "coturn_password".to_string()),
                realm: std::env::var("TURN_REALM")
                    .unwrap_or_else(|_| "linkwithmentor.com".to_string()),
                static_auth_secret: std::env::var("TURN_STATIC_AUTH_SECRET").ok(),
                ttl_seconds: std::env::var("TURN_TTL_SECONDS")
                    .unwrap_or_else(|_| "86400".to_string())
                    .parse()
                    .unwrap_or(86400),
            },
        })
    }
}