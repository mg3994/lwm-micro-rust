use linkwithmentor_common::{DatabaseConfig, RedisConfig, JwtConfig, ServerConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub safety: SafetyServiceConfig,
    pub ml: MLConfig,
    pub external_apis: ExternalApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyServiceConfig {
    pub enable_real_time_analysis: bool,
    pub enable_image_analysis: bool,
    pub enable_video_analysis: bool,
    pub auto_moderation_enabled: bool,
    pub toxicity_threshold: f32,
    pub spam_threshold: f32,
    pub hate_speech_threshold: f32,
    pub adult_content_threshold: f32,
    pub violence_threshold: f32,
    pub max_warnings_before_suspension: u32,
    pub suspension_duration_hours: u32,
    pub enable_appeal_process: bool,
    pub moderator_review_queue_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLConfig {
    pub model_cache_dir: String,
    pub toxicity_model_path: String,
    pub spam_model_path: String,
    pub hate_speech_model_path: String,
    pub image_classification_model_path: String,
    pub enable_gpu_acceleration: bool,
    pub batch_size: usize,
    pub max_text_length: usize,
    pub model_update_interval_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalApiConfig {
    pub perspective_api_key: Option<String>,
    pub aws_comprehend_region: Option<String>,
    pub azure_content_moderator_key: Option<String>,
    pub azure_content_moderator_endpoint: Option<String>,
    pub enable_external_validation: bool,
}

impl SafetyConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            server: ServerConfig {
                host: std::env::var("SAFETY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("SAFETY_PORT")
                    .unwrap_or_else(|_| "8007".to_string())
                    .parse()
                    .unwrap_or(8007),
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
                    .unwrap_or_else(|_| "4".to_string())
                    .parse()
                    .unwrap_or(4),
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
            safety: SafetyServiceConfig {
                enable_real_time_analysis: std::env::var("SAFETY_REAL_TIME_ANALYSIS")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                enable_image_analysis: std::env::var("SAFETY_IMAGE_ANALYSIS")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                enable_video_analysis: std::env::var("SAFETY_VIDEO_ANALYSIS")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                auto_moderation_enabled: std::env::var("SAFETY_AUTO_MODERATION")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                toxicity_threshold: std::env::var("SAFETY_TOXICITY_THRESHOLD")
                    .unwrap_or_else(|_| "0.7".to_string())
                    .parse()
                    .unwrap_or(0.7),
                spam_threshold: std::env::var("SAFETY_SPAM_THRESHOLD")
                    .unwrap_or_else(|_| "0.8".to_string())
                    .parse()
                    .unwrap_or(0.8),
                hate_speech_threshold: std::env::var("SAFETY_HATE_SPEECH_THRESHOLD")
                    .unwrap_or_else(|_| "0.6".to_string())
                    .parse()
                    .unwrap_or(0.6),
                adult_content_threshold: std::env::var("SAFETY_ADULT_CONTENT_THRESHOLD")
                    .unwrap_or_else(|_| "0.8".to_string())
                    .parse()
                    .unwrap_or(0.8),
                violence_threshold: std::env::var("SAFETY_VIOLENCE_THRESHOLD")
                    .unwrap_or_else(|_| "0.7".to_string())
                    .parse()
                    .unwrap_or(0.7),
                max_warnings_before_suspension: std::env::var("SAFETY_MAX_WARNINGS")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .unwrap_or(3),
                suspension_duration_hours: std::env::var("SAFETY_SUSPENSION_DURATION")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()
                    .unwrap_or(24),
                enable_appeal_process: std::env::var("SAFETY_ENABLE_APPEALS")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                moderator_review_queue_size: std::env::var("SAFETY_REVIEW_QUEUE_SIZE")
                    .unwrap_or_else(|_| "100".to_string())
                    .parse()
                    .unwrap_or(100),
            },
            ml: MLConfig {
                model_cache_dir: std::env::var("ML_MODEL_CACHE_DIR")
                    .unwrap_or_else(|_| "/app/models".to_string()),
                toxicity_model_path: std::env::var("ML_TOXICITY_MODEL_PATH")
                    .unwrap_or_else(|_| "models/toxicity".to_string()),
                spam_model_path: std::env::var("ML_SPAM_MODEL_PATH")
                    .unwrap_or_else(|_| "models/spam".to_string()),
                hate_speech_model_path: std::env::var("ML_HATE_SPEECH_MODEL_PATH")
                    .unwrap_or_else(|_| "models/hate_speech".to_string()),
                image_classification_model_path: std::env::var("ML_IMAGE_MODEL_PATH")
                    .unwrap_or_else(|_| "models/image_classification".to_string()),
                enable_gpu_acceleration: std::env::var("ML_ENABLE_GPU")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                batch_size: std::env::var("ML_BATCH_SIZE")
                    .unwrap_or_else(|_| "32".to_string())
                    .parse()
                    .unwrap_or(32),
                max_text_length: std::env::var("ML_MAX_TEXT_LENGTH")
                    .unwrap_or_else(|_| "512".to_string())
                    .parse()
                    .unwrap_or(512),
                model_update_interval_hours: std::env::var("ML_MODEL_UPDATE_INTERVAL")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()
                    .unwrap_or(24),
            },
            external_apis: ExternalApiConfig {
                perspective_api_key: std::env::var("PERSPECTIVE_API_KEY").ok(),
                aws_comprehend_region: std::env::var("AWS_COMPREHEND_REGION").ok(),
                azure_content_moderator_key: std::env::var("AZURE_CONTENT_MODERATOR_KEY").ok(),
                azure_content_moderator_endpoint: std::env::var("AZURE_CONTENT_MODERATOR_ENDPOINT").ok(),
                enable_external_validation: std::env::var("SAFETY_ENABLE_EXTERNAL_VALIDATION")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
            },
        })
    }
}