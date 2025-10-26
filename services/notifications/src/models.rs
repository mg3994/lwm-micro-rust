use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// Notification Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequest {
    pub recipient_id: Uuid,
    pub notification_type: NotificationType,
    pub channels: Vec<NotificationChannel>,
    pub title: String,
    pub message: String,
    pub template_id: Option<String>,
    pub template_data: Option<HashMap<String, serde_json::Value>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub priority: NotificationPriority,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub notification_id: Uuid,
    pub recipient_id: Uuid,
    pub status: NotificationStatus,
    pub channels: Vec<NotificationChannelStatus>,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannelStatus {
    pub channel: NotificationChannel,
    pub status: DeliveryStatus,
    pub sent_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: u32,
}

// Enums
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationType {
    SessionReminder,
    SessionStarted,
    SessionEnded,
    PaymentReceived,
    PaymentFailed,
    MessageReceived,
    MentorshipRequest,
    MentorshipAccepted,
    MentorshipRejected,
    ProfileUpdate,
    SecurityAlert,
    SystemMaintenance,
    Welcome,
    PasswordReset,
    EmailVerification,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationChannel {
    Email,
    SMS,
    Push,
    InApp,
    WebPush,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationStatus {
    Pending,
    Scheduled,
    Processing,
    Sent,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeliveryStatus {
    Pending,
    Sent,
    Delivered,
    Failed,
    Bounced,
    Clicked,
    Opened,
}

// Template Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTemplate {
    pub template_id: String,
    pub name: String,
    pub description: Option<String>,
    pub notification_type: NotificationType,
    pub channel: NotificationChannel,
    pub language: String,
    pub subject_template: Option<String>,
    pub body_template: String,
    pub variables: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Preference Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub user_id: Uuid,
    pub preferences: HashMap<NotificationType, ChannelPreferences>,
    pub quiet_hours_start: Option<String>,
    pub quiet_hours_end: Option<String>,
    pub timezone: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPreferences {
    pub enabled_channels: Vec<NotificationChannel>,
    pub disabled: bool,
}