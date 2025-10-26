use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use linkwithmentor_common::*;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub roles: Vec<String>, // PostgreSQL text array
    pub hashed_password: String,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PaymentMethodDb {
    pub payment_method_id: Uuid,
    pub user_id: Uuid,
    pub label: String,
    pub provider: String,
    pub vpa_address: String,
    pub is_primary: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Profile {
    pub user_id: Uuid,
    pub bio: Option<String>,
    pub payment_preferences: Option<serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MentorProfile {
    pub user_id: Uuid,
    pub specializations: serde_json::Value,
    pub hourly_rate: Decimal,
    pub availability: Option<serde_json::Value>,
    pub rating: Decimal,
    pub total_sessions_as_mentor: i32,
    pub years_of_experience: Option<i32>,
    pub certifications: Vec<String>,
    pub is_accepting_mentees: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MenteeProfile {
    pub user_id: Uuid,
    pub learning_goals: Option<serde_json::Value>,
    pub interests: Vec<String>,
    pub experience_level: String,
    pub total_sessions_as_mentee: i32,
    pub preferred_session_types: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MentorshipSession {
    pub session_id: Uuid,
    pub mentor_id: Uuid,
    pub mentee_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: DateTime<Utc>,
    pub actual_start: Option<DateTime<Utc>>,
    pub actual_end: Option<DateTime<Utc>>,
    pub status: String,
    pub session_type: String,
    pub whiteboard_data: Option<serde_json::Value>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatMessage {
    pub message_id: Uuid,
    pub session_id: Option<Uuid>,
    pub sender_id: Uuid,
    pub recipient_id: Option<Uuid>,
    pub group_id: Option<Uuid>,
    pub content: String,
    pub message_type: MessageType,
    pub moderation_status: ModerationStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub is_edited: bool,
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupChat {
    pub group_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_by: Uuid,
    pub session_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupChatParticipant {
    pub group_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub tx_id: Uuid,
    pub source_user_id: Uuid,
    pub target_user_id: Uuid,
    pub source_payment_method_id: Option<Uuid>,
    pub target_payment_method_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub amount: Decimal,
    pub currency: String,
    pub transaction_type: String,
    pub status: String,
    pub gateway_ref: Option<String>,
    pub service_fee: Decimal,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Subscription {
    pub subscription_id: Uuid,
    pub mentee_id: Uuid,
    pub mentor_id: Uuid,
    pub plan_type: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub auto_renew: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
}