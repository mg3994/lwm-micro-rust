use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use validator::Validate;
use linkwithmentor_common::{UserRole, PaymentProvider, ExperienceLevel};

// Request/Response DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 8))]
    pub password: String,
    
    pub roles: Vec<UserRole>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    
    pub password: String,
    
    pub active_role: Option<UserRole>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub roles: Vec<UserRole>,
    pub active_role: Option<UserRole>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateProfileRequest {
    pub bio: Option<String>,
    pub payment_preferences: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateMentorProfileRequest {
    pub specializations: serde_json::Value,
    
    #[validate(range(min = 0.01))]
    pub hourly_rate: rust_decimal::Decimal,
    
    pub availability: Option<serde_json::Value>,
    pub years_of_experience: Option<i32>,
    pub certifications: Vec<String>,
    pub is_accepting_mentees: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateMentorProfileRequest {
    pub specializations: Option<serde_json::Value>,
    pub hourly_rate: Option<rust_decimal::Decimal>,
    pub availability: Option<serde_json::Value>,
    pub years_of_experience: Option<i32>,
    pub certifications: Option<Vec<String>>,
    pub is_accepting_mentees: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateMenteeProfileRequest {
    pub learning_goals: Option<serde_json::Value>,
    pub interests: Vec<String>,
    pub experience_level: ExperienceLevel,
    pub preferred_session_types: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateMenteeProfileRequest {
    pub learning_goals: Option<serde_json::Value>,
    pub interests: Option<Vec<String>>,
    pub experience_level: Option<ExperienceLevel>,
    pub preferred_session_types: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct AddPaymentMethodRequest {
    #[validate(length(min = 1, max = 100))]
    pub label: String,
    
    pub provider: PaymentProvider,
    
    #[validate(length(min = 1, max = 255))]
    pub vpa_address: String,
    
    pub is_primary: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdatePaymentMethodRequest {
    pub label: Option<String>,
    pub vpa_address: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentMethodResponse {
    pub payment_method_id: Uuid,
    pub label: String,
    pub provider: PaymentProvider,
    pub vpa_address: String,
    pub is_primary: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MentorProfileResponse {
    pub user_id: Uuid,
    pub specializations: serde_json::Value,
    pub hourly_rate: rust_decimal::Decimal,
    pub availability: Option<serde_json::Value>,
    pub rating: rust_decimal::Decimal,
    pub total_sessions_as_mentor: i32,
    pub years_of_experience: Option<i32>,
    pub certifications: Vec<String>,
    pub is_accepting_mentees: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MenteeProfileResponse {
    pub user_id: Uuid,
    pub learning_goals: Option<serde_json::Value>,
    pub interests: Vec<String>,
    pub experience_level: String,
    pub total_sessions_as_mentee: i32,
    pub preferred_session_types: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileResponse {
    pub user_id: Uuid,
    pub bio: Option<String>,
    pub payment_preferences: Option<serde_json::Value>,
    pub mentor_profile: Option<MentorProfileResponse>,
    pub mentee_profile: Option<MenteeProfileResponse>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct PasswordResetRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct PasswordResetConfirmRequest {
    pub token: String,
    
    #[validate(length(min = 8))]
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    
    #[validate(length(min = 8))]
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleSwitchRequest {
    pub new_role: UserRole,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleOAuthRequest {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
    pub verified_email: bool,
}