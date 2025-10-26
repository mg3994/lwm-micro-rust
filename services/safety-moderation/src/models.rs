use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// Content Analysis Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAnalysisRequest {
    pub content: String,
    pub content_type: ContentType,
    pub context: Option<ContentContext>,
    pub user_id: Uuid,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAnalysisResponse {
    pub analysis_id: Uuid,
    pub content_hash: String,
    pub scores: ModerationScores,
    pub violations: Vec<PolicyViolation>,
    pub recommended_action: ModerationAction,
    pub confidence: f32,
    pub processed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationScores {
    pub toxicity: f32,
    pub spam: f32,
    pub hate_speech: f32,
    pub harassment: f32,
    pub adult_content: f32,
    pub violence: f32,
    pub self_harm: f32,
    pub overall_risk: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    pub policy_type: PolicyType,
    pub severity: ViolationSeverity,
    pub confidence: f32,
    pub description: String,
    pub evidence: Vec<String>,
}

// Image Analysis Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAnalysisRequest {
    pub image_url: String,
    pub image_data: Option<Vec<u8>>,
    pub user_id: Uuid,
    pub context: Option<ContentContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAnalysisResponse {
    pub analysis_id: Uuid,
    pub image_hash: String,
    pub classifications: Vec<ImageClassification>,
    pub adult_content_score: f32,
    pub violence_score: f32,
    pub racy_content_score: f32,
    pub recommended_action: ModerationAction,
    pub processed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageClassification {
    pub label: String,
    pub confidence: f32,
    pub bounding_box: Option<BoundingBox>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

// Moderation Action Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationActionRequest {
    pub target_type: ModerationTargetType,
    pub target_id: Uuid,
    pub action: ModerationAction,
    pub reason: String,
    pub moderator_id: Option<Uuid>,
    pub duration_hours: Option<u32>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationActionResponse {
    pub action_id: Uuid,
    pub target_type: ModerationTargetType,
    pub target_id: Uuid,
    pub action: ModerationAction,
    pub status: ActionStatus,
    pub reason: String,
    pub moderator_id: Option<Uuid>,
    pub automated: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// Reporting Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRequest {
    pub reported_content_type: ContentType,
    pub reported_content_id: Uuid,
    pub reported_user_id: Uuid,
    pub report_type: ReportType,
    pub description: String,
    pub evidence: Vec<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportResponse {
    pub report_id: Uuid,
    pub reporter_id: Uuid,
    pub reported_user_id: Uuid,
    pub report_type: ReportType,
    pub status: ReportStatus,
    pub priority: ReportPriority,
    pub assigned_moderator: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Investigation Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationRequest {
    pub report_id: Uuid,
    pub moderator_id: Uuid,
    pub investigation_notes: String,
    pub evidence_collected: Vec<String>,
    pub recommended_action: ModerationAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationResponse {
    pub investigation_id: Uuid,
    pub report_id: Uuid,
    pub moderator_id: Uuid,
    pub status: InvestigationStatus,
    pub findings: String,
    pub evidence: Vec<Evidence>,
    pub recommended_action: ModerationAction,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_id: Uuid,
    pub evidence_type: EvidenceType,
    pub content: String,
    pub source: String,
    pub collected_at: DateTime<Utc>,
}

// Appeal Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppealRequest {
    pub action_id: Uuid,
    pub appeal_reason: String,
    pub additional_context: Option<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppealResponse {
    pub appeal_id: Uuid,
    pub action_id: Uuid,
    pub user_id: Uuid,
    pub status: AppealStatus,
    pub reason: String,
    pub reviewer_id: Option<Uuid>,
    pub decision: Option<AppealDecision>,
    pub decision_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

// User Safety Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSafetyProfile {
    pub user_id: Uuid,
    pub risk_score: f32,
    pub warning_count: u32,
    pub suspension_count: u32,
    pub ban_count: u32,
    pub last_violation: Option<DateTime<Utc>>,
    pub account_status: AccountStatus,
    pub restrictions: Vec<UserRestriction>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRestriction {
    pub restriction_type: RestrictionType,
    pub reason: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// Analytics Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationAnalytics {
    pub total_content_analyzed: u64,
    pub violations_detected: u64,
    pub automated_actions: u64,
    pub manual_reviews: u64,
    pub appeals_submitted: u64,
    pub appeals_upheld: u64,
    pub top_violation_types: Vec<ViolationTypeStats>,
    pub moderation_accuracy: f32,
    pub average_response_time_hours: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationTypeStats {
    pub violation_type: PolicyType,
    pub count: u64,
    pub percentage: f32,
}

// Enums
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentType {
    Text,
    Image,
    Video,
    Audio,
    Document,
    Link,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentContext {
    pub platform_area: String, // chat, profile, session, etc.
    pub session_id: Option<Uuid>,
    pub conversation_id: Option<Uuid>,
    pub parent_content_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyType {
    Toxicity,
    Spam,
    HateSpeech,
    Harassment,
    AdultContent,
    Violence,
    SelfHarm,
    Misinformation,
    Copyright,
    Privacy,
    Fraud,
    Impersonation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModerationAction {
    NoAction,
    Warning,
    ContentRemoval,
    ContentHide,
    UserSuspension,
    UserBan,
    AccountRestriction,
    RequireReview,
    Escalate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModerationTargetType {
    User,
    Content,
    Session,
    Profile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionStatus {
    Pending,
    Active,
    Completed,
    Reversed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportType {
    InappropriateContent,
    Harassment,
    Spam,
    HateSpeech,
    Violence,
    AdultContent,
    Impersonation,
    Fraud,
    Copyright,
    Privacy,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportStatus {
    Submitted,
    UnderReview,
    InvestigationRequired,
    Resolved,
    Dismissed,
    Escalated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InvestigationStatus {
    Assigned,
    InProgress,
    Completed,
    Escalated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvidenceType {
    Screenshot,
    TextContent,
    UserProfile,
    ConversationHistory,
    SystemLogs,
    ExternalLink,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AppealStatus {
    Submitted,
    UnderReview,
    Approved,
    Denied,
    Escalated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AppealDecision {
    Upheld,
    Overturned,
    Modified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccountStatus {
    Active,
    Warned,
    Restricted,
    Suspended,
    Banned,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RestrictionType {
    ChatRestriction,
    SessionRestriction,
    ProfileRestriction,
    PaymentRestriction,
    UploadRestriction,
}

// Database Models
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ContentAnalysisDb {
    pub analysis_id: Uuid,
    pub content_hash: String,
    pub user_id: Uuid,
    pub content_type: String,
    pub scores: serde_json::Value,
    pub violations: serde_json::Value,
    pub recommended_action: String,
    pub confidence: f32,
    pub processed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ModerationActionDb {
    pub action_id: Uuid,
    pub target_type: String,
    pub target_id: Uuid,
    pub action: String,
    pub status: String,
    pub reason: String,
    pub moderator_id: Option<Uuid>,
    pub automated: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReportDb {
    pub report_id: Uuid,
    pub reporter_id: Uuid,
    pub reported_user_id: Uuid,
    pub reported_content_type: String,
    pub reported_content_id: Uuid,
    pub report_type: String,
    pub status: String,
    pub priority: String,
    pub description: String,
    pub evidence: serde_json::Value,
    pub assigned_moderator: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Error Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyError {
    AnalysisError(String),
    ModelLoadError(String),
    InvalidContent,
    RateLimitExceeded,
    InsufficientPermissions,
    ResourceNotFound,
    ExternalApiError(String),
    DatabaseError(String),
}

impl std::fmt::Display for SafetyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SafetyError::AnalysisError(msg) => write!(f, "Content analysis error: {}", msg),
            SafetyError::ModelLoadError(msg) => write!(f, "ML model error: {}", msg),
            SafetyError::InvalidContent => write!(f, "Invalid content provided"),
            SafetyError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            SafetyError::InsufficientPermissions => write!(f, "Insufficient permissions"),
            SafetyError::ResourceNotFound => write!(f, "Resource not found"),
            SafetyError::ExternalApiError(msg) => write!(f, "External API error: {}", msg),
            SafetyError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for SafetyError {}