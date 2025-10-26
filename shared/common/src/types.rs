use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Mentee,
    Mentor,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentProvider {
    UPI,
    PayPal,
    GooglePay,
    Stripe,
    Razorpay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub payment_method_id: Uuid,
    pub label: String,
    pub provider: PaymentProvider,
    pub vpa_address: String,
    pub is_primary: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExperienceLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    OneTime,
    Recurring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Scheduled,
    InProgress,
    Completed,
    Cancelled,
    NoShow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Subscription,
    SessionPayment,
    HourlyPayment,
    Payout,
    Refund,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionPlan {
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Active,
    Cancelled,
    Expired,
    Suspended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text,
    Image,
    File,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModerationStatus {
    Pending,
    Approved,
    Flagged,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallType {
    Audio,
    Video,
    ScreenShare,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallStatus {
    Initiating,
    Ringing,
    Connected,
    Ended,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Image,
    Video,
    Audio,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeverityLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyType {
    InappropriateLanguage,
    Harassment,
    Spam,
    Violence,
    AdultContent,
    Copyright,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModerationAction {
    None,
    Warning,
    ContentRemoval,
    TemporaryBan,
    PermanentBan,
}

// Common response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: Utc::now(),
        }
    }
}