use linkwithmentor_common::{DatabaseConfig, RedisConfig, JwtConfig, ServerConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub notification: NotificationServiceConfig,
    pub email: EmailConfig,
    pub sms: SmsConfig,
    pub push: PushConfig,
    pub templates: TemplateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationServiceConfig {
    pub max_retry_attempts: u32,
    pub retry_delay_seconds: u64,
    pub batch_size: u32,
    pub worker_count: u32,
    pub rate_limit_per_minute: u32,
    pub enable_delivery_tracking: bool,
    pub default_timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_encryption: String, // none, tls, starttls
    pub from_email: String,
    pub from_name: String,
    pub reply_to: Option<String>,
    pub max_recipients: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConfig {
    pub enabled: bool,
    pub provider: String, // twilio, aws_sns, etc.
    pub api_key: String,
    pub api_secret: String,
    pub from_number: String,
    pub webhook_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushConfig {
    pub enabled: bool,
    pub fcm_server_key: String,
    pub fcm_sender_id: String,
    pub apns_key_id: String,
    pub apns_team_id: String,
    pub apns_bundle_id: String,
    pub apns_key_path: String,
    pub web_push_vapid_public_key: String,
    pub web_push_vapid_private_key: String,
    pub web_push_contact_email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub template_dir: String,
    pub cache_templates: bool,
    pub default_language: String,
    pub supported_languages: Vec<String>,
}

impl NotificationConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            server: ServerConfig {
                host: std::env::var("NOTIFICATIONS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("NOTIFICATIONS_PORT")
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
            notification: NotificationServiceConfig {
                max_retry_attempts: std::env::var("NOTIFICATION_MAX_RETRIES")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .unwrap_or(3),
                retry_delay_seconds: std::env::var("NOTIFICATION_RETRY_DELAY")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()
                    .unwrap_or(60),
                batch_size: std::env::var("NOTIFICATION_BATCH_SIZE")
                    .unwrap_or_else(|_| "100".to_string())
                    .parse()
                    .unwrap_or(100),
                worker_count: std::env::var("NOTIFICATION_WORKERS")
                    .unwrap_or_else(|_| "4".to_string())
                    .parse()
                    .unwrap_or(4),
                rate_limit_per_minute: std::env::var("NOTIFICATION_RATE_LIMIT")
                    .unwrap_or_else(|_| "1000".to_string())
                    .parse()
                    .unwrap_or(1000),
                enable_delivery_tracking: std::env::var("NOTIFICATION_TRACK_DELIVERY")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                default_timezone: std::env::var("DEFAULT_TIMEZONE")
                    .unwrap_or_else(|_| "UTC".to_string()),
            },
            email: EmailConfig {
                enabled: std::env::var("EMAIL_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                smtp_host: std::env::var("SMTP_HOST")
                    .unwrap_or_else(|_| "smtp.gmail.com".to_string()),
                smtp_port: std::env::var("SMTP_PORT")
                    .unwrap_or_else(|_| "587".to_string())
                    .parse()
                    .unwrap_or(587),
                smtp_username: std::env::var("SMTP_USERNAME")
                    .unwrap_or_else(|_| "noreply@linkwithmentor.com".to_string()),
                smtp_password: std::env::var("SMTP_PASSWORD")
                    .unwrap_or_else(|_| "smtp_password".to_string()),
                smtp_encryption: std::env::var("SMTP_ENCRYPTION")
                    .unwrap_or_else(|_| "starttls".to_string()),
                from_email: std::env::var("FROM_EMAIL")
                    .unwrap_or_else(|_| "noreply@linkwithmentor.com".to_string()),
                from_name: std::env::var("FROM_NAME")
                    .unwrap_or_else(|_| "LinkWithMentor".to_string()),
                reply_to: std::env::var("REPLY_TO_EMAIL").ok(),
                max_recipients: std::env::var("EMAIL_MAX_RECIPIENTS")
                    .unwrap_or_else(|_| "50".to_string())
                    .parse()
                    .unwrap_or(50),
            },
            sms: SmsConfig {
                enabled: std::env::var("SMS_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                provider: std::env::var("SMS_PROVIDER")
                    .unwrap_or_else(|_| "twilio".to_string()),
                api_key: std::env::var("SMS_API_KEY")
                    .unwrap_or_else(|_| "sms_api_key".to_string()),
                api_secret: std::env::var("SMS_API_SECRET")
                    .unwrap_or_else(|_| "sms_api_secret".to_string()),
                from_number: std::env::var("SMS_FROM_NUMBER")
                    .unwrap_or_else(|_| "+1234567890".to_string()),
                webhook_url: std::env::var("SMS_WEBHOOK_URL")
                    .unwrap_or_else(|_| "https://api.linkwithmentor.com/webhooks/sms".to_string()),
            },
            push: PushConfig {
                enabled: std::env::var("PUSH_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                fcm_server_key: std::env::var("FCM_SERVER_KEY")
                    .unwrap_or_else(|_| "fcm_server_key".to_string()),
                fcm_sender_id: std::env::var("FCM_SENDER_ID")
                    .unwrap_or_else(|_| "fcm_sender_id".to_string()),
                apns_key_id: std::env::var("APNS_KEY_ID")
                    .unwrap_or_else(|_| "apns_key_id".to_string()),
                apns_team_id: std::env::var("APNS_TEAM_ID")
                    .unwrap_or_else(|_| "apns_team_id".to_string()),
                apns_bundle_id: std::env::var("APNS_BUNDLE_ID")
                    .unwrap_or_else(|_| "com.linkwithmentor.app".to_string()),
                apns_key_path: std::env::var("APNS_KEY_PATH")
                    .unwrap_or_else(|_| "/app/keys/apns.p8".to_string()),
                web_push_vapid_public_key: std::env::var("WEB_PUSH_VAPID_PUBLIC_KEY")
                    .unwrap_or_else(|_| "vapid_public_key".to_string()),
                web_push_vapid_private_key: std::env::var("WEB_PUSH_VAPID_PRIVATE_KEY")
                    .unwrap_or_else(|_| "vapid_private_key".to_string()),
                web_push_contact_email: std::env::var("WEB_PUSH_CONTACT_EMAIL")
                    .unwrap_or_else(|_| "admin@linkwithmentor.com".to_string()),
            },
            templates: TemplateConfig {
                template_dir: std::env::var("TEMPLATE_DIR")
                    .unwrap_or_else(|_| "./templates".to_string()),
                cache_templates: std::env::var("CACHE_TEMPLATES")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                default_language: std::env::var("DEFAULT_LANGUAGE")
                    .unwrap_or_else(|_| "en".to_string()),
                supported_languages: std::env::var("SUPPORTED_LANGUAGES")
                    .unwrap_or_else(|_| "en,hi,es,fr".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            },
        })
    }
}