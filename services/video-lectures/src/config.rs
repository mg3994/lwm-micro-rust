use linkwithmentor_common::{DatabaseConfig, RedisConfig, JwtConfig, ServerConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoLecturesConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub upload: UploadConfig,
    pub processing: ProcessingConfig,
    pub streaming: StreamingConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadConfig {
    pub max_file_size_mb: u64,
    pub allowed_formats: Vec<String>,
    pub upload_timeout_seconds: u64,
    pub chunk_size_mb: u64,
    pub temp_storage_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    pub ffmpeg_path: String,
    pub output_formats: Vec<VideoFormat>,
    pub thumbnail_count: u32,
    pub thumbnail_width: u32,
    pub thumbnail_height: u32,
    pub max_concurrent_jobs: u32,
    pub processing_timeout_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFormat {
    pub name: String,
    pub resolution: String,
    pub bitrate: String,
    pub codec: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    pub hls_segment_duration: u32,
    pub dash_segment_duration: u32,
    pub adaptive_bitrates: Vec<String>,
    pub cdn_base_url: String,
    pub enable_drm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub provider: String, // "s3", "local", "gcs"
    pub bucket_name: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub cdn_domain: Option<String>,
    pub local_storage_path: String,
}

impl VideoLecturesConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            server: ServerConfig {
                host: std::env::var("VIDEO_LECTURES_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("VIDEO_LECTURES_PORT")
                    .unwrap_or_else(|_| "8006".to_string())
                    .parse()
                    .unwrap_or(8006),
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
            upload: UploadConfig {
                max_file_size_mb: std::env::var("VIDEO_MAX_FILE_SIZE_MB")
                    .unwrap_or_else(|_| "500".to_string())
                    .parse()
                    .unwrap_or(500),
                allowed_formats: std::env::var("VIDEO_ALLOWED_FORMATS")
                    .unwrap_or_else(|_| "mp4,avi,mov,mkv,webm".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                upload_timeout_seconds: std::env::var("VIDEO_UPLOAD_TIMEOUT")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse()
                    .unwrap_or(3600),
                chunk_size_mb: std::env::var("VIDEO_CHUNK_SIZE_MB")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
                temp_storage_path: std::env::var("VIDEO_TEMP_STORAGE")
                    .unwrap_or_else(|_| "/tmp/video-uploads".to_string()),
            },
            processing: ProcessingConfig {
                ffmpeg_path: std::env::var("FFMPEG_PATH")
                    .unwrap_or_else(|_| "ffmpeg".to_string()),
                output_formats: vec![
                    VideoFormat {
                        name: "720p".to_string(),
                        resolution: "1280x720".to_string(),
                        bitrate: "2500k".to_string(),
                        codec: "libx264".to_string(),
                    },
                    VideoFormat {
                        name: "480p".to_string(),
                        resolution: "854x480".to_string(),
                        bitrate: "1000k".to_string(),
                        codec: "libx264".to_string(),
                    },
                    VideoFormat {
                        name: "360p".to_string(),
                        resolution: "640x360".to_string(),
                        bitrate: "500k".to_string(),
                        codec: "libx264".to_string(),
                    },
                ],
                thumbnail_count: std::env::var("VIDEO_THUMBNAIL_COUNT")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()
                    .unwrap_or(5),
                thumbnail_width: std::env::var("VIDEO_THUMBNAIL_WIDTH")
                    .unwrap_or_else(|_| "320".to_string())
                    .parse()
                    .unwrap_or(320),
                thumbnail_height: std::env::var("VIDEO_THUMBNAIL_HEIGHT")
                    .unwrap_or_else(|_| "180".to_string())
                    .parse()
                    .unwrap_or(180),
                max_concurrent_jobs: std::env::var("VIDEO_MAX_CONCURRENT_JOBS")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .unwrap_or(3),
                processing_timeout_minutes: std::env::var("VIDEO_PROCESSING_TIMEOUT")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()
                    .unwrap_or(60),
            },
            streaming: StreamingConfig {
                hls_segment_duration: std::env::var("VIDEO_HLS_SEGMENT_DURATION")
                    .unwrap_or_else(|_| "6".to_string())
                    .parse()
                    .unwrap_or(6),
                dash_segment_duration: std::env::var("VIDEO_DASH_SEGMENT_DURATION")
                    .unwrap_or_else(|_| "4".to_string())
                    .parse()
                    .unwrap_or(4),
                adaptive_bitrates: std::env::var("VIDEO_ADAPTIVE_BITRATES")
                    .unwrap_or_else(|_| "500k,1000k,2500k".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                cdn_base_url: std::env::var("VIDEO_CDN_BASE_URL")
                    .unwrap_or_else(|_| "https://cdn.linkwithmentor.com".to_string()),
                enable_drm: std::env::var("VIDEO_ENABLE_DRM")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
            },
            storage: StorageConfig {
                provider: std::env::var("VIDEO_STORAGE_PROVIDER")
                    .unwrap_or_else(|_| "s3".to_string()),
                bucket_name: std::env::var("VIDEO_STORAGE_BUCKET")
                    .unwrap_or_else(|_| "linkwithmentor-videos".to_string()),
                region: std::env::var("VIDEO_STORAGE_REGION")
                    .unwrap_or_else(|_| "us-east-1".to_string()),
                access_key: std::env::var("VIDEO_STORAGE_ACCESS_KEY")
                    .unwrap_or_else(|_| "".to_string()),
                secret_key: std::env::var("VIDEO_STORAGE_SECRET_KEY")
                    .unwrap_or_else(|_| "".to_string()),
                cdn_domain: std::env::var("VIDEO_CDN_DOMAIN").ok(),
                local_storage_path: std::env::var("VIDEO_LOCAL_STORAGE")
                    .unwrap_or_else(|_| "/app/videos".to_string()),
            },
        })
    }
}