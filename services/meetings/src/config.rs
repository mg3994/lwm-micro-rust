use linkwithmentor_common::{DatabaseConfig, RedisConfig, JwtConfig, ServerConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingsConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub meetings: MeetingsServiceConfig,
    pub notifications: NotificationConfig,
    pub calendar: CalendarConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingsServiceConfig {
    pub max_session_duration_hours: u32,
    pub default_session_duration_minutes: u32,
    pub advance_booking_days: u32,
    pub cancellation_window_hours: u32,
    pub reminder_intervals_minutes: Vec<u32>,
    pub max_participants_per_session: u32,
    pub enable_recurring_sessions: bool,
    pub max_recurring_sessions: u32,
    pub whiteboard_storage_path: String,
    pub session_materials_path: String,
    pub enable_session_recording: bool,
    pub auto_save_interval_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_email: String,
    pub from_name: String,
    pub enable_email_notifications: bool,
    pub enable_sms_notifications: bool,
    pub sms_provider: String,
    pub sms_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarConfig {
    pub enable_ical_export: bool,
    pub calendar_name: String,
    pub timezone: String,
    pub enable_google_calendar: bool,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub enable_outlook_calendar: bool,
    pub outlook_client_id: Option<String>,
    pub outlook_client_secret: Option<String>,
}

impl MeetingsConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            server: ServerConfig {
                host: std::env::var("MEETINGS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("MEETINGS_PORT")
                    .unwrap_or_else(|_| "8004".to_string())
                    .parse()
                    .unwrap_or(8004),
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
                    .unwrap_or_else(|_| "2".to_string())
                    .parse()
                    .unwrap_or(2),
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
            meetings: MeetingsServiceConfig {
                max_session_duration_hours: std::env::var("MEETINGS_MAX_DURATION_HOURS")
                    .unwrap_or_else(|_| "4".to_string())
                    .parse()
                    .unwrap_or(4),
                default_session_duration_minutes: std::env::var("MEETINGS_DEFAULT_DURATION")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()
                    .unwrap_or(60),
                advance_booking_days: std::env::var("MEETINGS_ADVANCE_BOOKING_DAYS")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
                cancellation_window_hours: std::env::var("MEETINGS_CANCELLATION_WINDOW")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()
                    .unwrap_or(24),
                reminder_intervals_minutes: std::env::var("MEETINGS_REMINDER_INTERVALS")
                    .unwrap_or_else(|_| "1440,60,15".to_string()) // 24h, 1h, 15min
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect(),
                max_participants_per_session: std::env::var("MEETINGS_MAX_PARTICIPANTS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
                enable_recurring_sessions: std::env::var("MEETINGS_ENABLE_RECURRING")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                max_recurring_sessions: std::env::var("MEETINGS_MAX_RECURRING")
                    .unwrap_or_else(|_| "52".to_string())
                    .parse()
                    .unwrap_or(52),
                whiteboard_storage_path: std::env::var("MEETINGS_WHITEBOARD_PATH")
                    .unwrap_or_else(|_| "/app/whiteboards".to_string()),
                session_materials_path: std::env::var("MEETINGS_MATERIALS_PATH")
                    .unwrap_or_else(|_| "/app/materials".to_string()),
                enable_session_recording: std::env::var("MEETINGS_ENABLE_RECORDING")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                auto_save_interval_seconds: std::env::var("MEETINGS_AUTO_SAVE_INTERVAL")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
            },
            notifications: NotificationConfig {
                smtp_host: std::env::var("SMTP_HOST")
                    .unwrap_or_else(|_| "localhost".to_string()),
                smtp_port: std::env::var("SMTP_PORT")
                    .unwrap_or_else(|_| "587".to_string())
                    .parse()
                    .unwrap_or(587),
                smtp_username: std::env::var("SMTP_USERNAME")
                    .unwrap_or_else(|_| "noreply@linkwithmentor.com".to_string()),
                smtp_password: std::env::var("SMTP_PASSWORD")
                    .unwrap_or_else(|_| "password".to_string()),
                from_email: std::env::var("FROM_EMAIL")
                    .unwrap_or_else(|_| "noreply@linkwithmentor.com".to_string()),
                from_name: std::env::var("FROM_NAME")
                    .unwrap_or_else(|_| "LinkWithMentor".to_string()),
                enable_email_notifications: std::env::var("ENABLE_EMAIL_NOTIFICATIONS")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                enable_sms_notifications: std::env::var("ENABLE_SMS_NOTIFICATIONS")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                sms_provider: std::env::var("SMS_PROVIDER")
                    .unwrap_or_else(|_| "twilio".to_string()),
                sms_api_key: std::env::var("SMS_API_KEY").ok(),
            },
            calendar: CalendarConfig {
                enable_ical_export: std::env::var("CALENDAR_ENABLE_ICAL")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                calendar_name: std::env::var("CALENDAR_NAME")
                    .unwrap_or_else(|_| "LinkWithMentor Sessions".to_string()),
                timezone: std::env::var("CALENDAR_TIMEZONE")
                    .unwrap_or_else(|_| "UTC".to_string()),
                enable_google_calendar: std::env::var("CALENDAR_ENABLE_GOOGLE")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                google_client_id: std::env::var("GOOGLE_CLIENT_ID").ok(),
                google_client_secret: std::env::var("GOOGLE_CLIENT_SECRET").ok(),
                enable_outlook_calendar: std::env::var("CALENDAR_ENABLE_OUTLOOK")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                outlook_client_id: std::env::var("OUTLOOK_CLIENT_ID").ok(),
                outlook_client_secret: std::env::var("OUTLOOK_CLIENT_SECRET").ok(),
            },
        })
    }
}