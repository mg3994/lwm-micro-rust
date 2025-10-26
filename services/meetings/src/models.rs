use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveTime};
use std::collections::HashMap;

// Session Management Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRequest {
    pub mentor_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub scheduled_start: DateTime<Utc>,
    pub duration_minutes: u32,
    pub session_type: SessionType,
    pub recurring_pattern: Option<RecurringPattern>,
    pub max_participants: Option<u32>,
    pub materials: Vec<SessionMaterial>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub session_id: Uuid,
    pub mentor_id: Uuid,
    pub mentee_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: DateTime<Utc>,
    pub actual_start: Option<DateTime<Utc>>,
    pub actual_end: Option<DateTime<Utc>>,
    pub status: SessionStatus,
    pub session_type: SessionType,
    pub participants: Vec<SessionParticipant>,
    pub materials: Vec<SessionMaterial>,
    pub whiteboard_id: Option<Uuid>,
    pub notes: Option<String>,
    pub recurring_series_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSessionRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub scheduled_start: Option<DateTime<Utc>>,
    pub duration_minutes: Option<u32>,
    pub status: Option<SessionStatus>,
    pub notes: Option<String>,
}

// Session Types and Status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionType {
    OneOnOne,
    GroupSession,
    Workshop,
    Consultation,
    FollowUp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Scheduled,
    Confirmed,
    InProgress,
    Completed,
    Cancelled,
    NoShow,
    Rescheduled,
}

// Recurring Sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringPattern {
    pub frequency: RecurrenceFrequency,
    pub interval: u32, // Every N weeks/months
    pub days_of_week: Option<Vec<u8>>, // 0=Sunday, 1=Monday, etc.
    pub end_date: Option<DateTime<Utc>>,
    pub max_occurrences: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringSeriesResponse {
    pub series_id: Uuid,
    pub mentor_id: Uuid,
    pub title: String,
    pub pattern: RecurringPattern,
    pub sessions: Vec<SessionResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Participants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionParticipant {
    pub user_id: Uuid,
    pub username: String,
    pub role: ParticipantRole,
    pub status: ParticipantStatus,
    pub joined_at: Option<DateTime<Utc>>,
    pub left_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticipantRole {
    Mentor,
    Mentee,
    Observer,
    Guest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticipantStatus {
    Invited,
    Confirmed,
    Declined,
    Attended,
    NoShow,
}

// Availability Management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityRequest {
    pub day_of_week: u8, // 0=Sunday, 1=Monday, etc.
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub timezone: String,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityResponse {
    pub availability_id: Uuid,
    pub user_id: Uuid,
    pub day_of_week: u8,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub timezone: String,
    pub is_available: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilitySlot {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub is_available: bool,
    pub existing_session_id: Option<Uuid>,
}

// Session Materials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMaterial {
    pub material_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub material_type: MaterialType,
    pub uploaded_by: Uuid,
    pub uploaded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MaterialType {
    Document,
    Presentation,
    Video,
    Audio,
    Image,
    Link,
    Note,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMaterialRequest {
    pub name: String,
    pub description: Option<String>,
    pub material_type: MaterialType,
    pub content: Option<String>, // For notes/links
}

// Whiteboard Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhiteboardState {
    pub whiteboard_id: Uuid,
    pub session_id: Uuid,
    pub elements: Vec<WhiteboardElement>,
    pub version: u64,
    pub last_modified: DateTime<Utc>,
    pub last_modified_by: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhiteboardElement {
    pub element_id: Uuid,
    pub element_type: ElementType,
    pub position: Position,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElementType {
    Text,
    Shape,
    Line,
    FreeDrawing,
    Image,
    Sticky,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhiteboardOperation {
    pub operation_type: OperationType,
    pub element_id: Option<Uuid>,
    pub element: Option<WhiteboardElement>,
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Create,
    Update,
    Delete,
    Move,
    Clear,
}

// Collaboration Models
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CollaborationMessage {
    // Whiteboard operations
    WhiteboardUpdate {
        whiteboard_id: Uuid,
        operation: WhiteboardOperation,
    },
    WhiteboardSync {
        whiteboard_id: Uuid,
        state: WhiteboardState,
    },
    
    // Cursor tracking
    CursorUpdate {
        user_id: Uuid,
        username: String,
        position: Position,
        color: String,
    },
    
    // User presence
    UserJoined {
        user_id: Uuid,
        username: String,
        role: ParticipantRole,
    },
    UserLeft {
        user_id: Uuid,
        username: String,
    },
    
    // Session control
    SessionStarted {
        session_id: Uuid,
        started_by: Uuid,
    },
    SessionEnded {
        session_id: Uuid,
        ended_by: Uuid,
    },
    
    // Screen sharing
    ScreenShareStarted {
        user_id: Uuid,
        username: String,
    },
    ScreenShareStopped {
        user_id: Uuid,
        username: String,
    },
    
    // Chat messages
    ChatMessage {
        message_id: Uuid,
        sender_id: Uuid,
        sender_username: String,
        content: String,
        timestamp: DateTime<Utc>,
    },
    
    // Error handling
    Error {
        code: String,
        message: String,
    },
}

// Notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequest {
    pub recipient_id: Uuid,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub session_id: Option<Uuid>,
    pub scheduled_for: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    SessionReminder,
    SessionConfirmation,
    SessionCancellation,
    SessionRescheduled,
    SessionStarted,
    SessionEnded,
    MaterialShared,
    InvitationReceived,
}

// Calendar Integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub event_id: String,
    pub session_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub location: Option<String>,
    pub attendees: Vec<String>,
    pub calendar_provider: CalendarProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CalendarProvider {
    Google,
    Outlook,
    Apple,
    ICalendar,
}

// Analytics and Reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAnalytics {
    pub session_id: Uuid,
    pub duration_minutes: u32,
    pub participant_count: u32,
    pub whiteboard_elements_created: u32,
    pub materials_shared: u32,
    pub chat_messages: u32,
    pub engagement_score: f32,
    pub completion_status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentorAnalytics {
    pub mentor_id: Uuid,
    pub total_sessions: u32,
    pub completed_sessions: u32,
    pub cancelled_sessions: u32,
    pub no_show_sessions: u32,
    pub average_session_duration: f32,
    pub average_rating: f32,
    pub total_mentees: u32,
    pub recurring_sessions: u32,
}

// Database Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDb {
    pub session_id: Uuid,
    pub mentor_id: Uuid,
    pub mentee_id: Option<Uuid>,
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
    pub recurring_series_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Error Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeetingsError {
    SessionNotFound,
    ConflictingSchedule,
    InvalidTimeSlot,
    MaxParticipantsReached,
    PermissionDenied,
    RecurringSeriesNotFound,
    WhiteboardNotFound,
    MaterialNotFound,
    NotificationFailed,
    CalendarSyncFailed,
}

impl std::fmt::Display for MeetingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeetingsError::SessionNotFound => write!(f, "Session not found"),
            MeetingsError::ConflictingSchedule => write!(f, "Conflicting schedule"),
            MeetingsError::InvalidTimeSlot => write!(f, "Invalid time slot"),
            MeetingsError::MaxParticipantsReached => write!(f, "Maximum participants reached"),
            MeetingsError::PermissionDenied => write!(f, "Permission denied"),
            MeetingsError::RecurringSeriesNotFound => write!(f, "Recurring series not found"),
            MeetingsError::WhiteboardNotFound => write!(f, "Whiteboard not found"),
            MeetingsError::MaterialNotFound => write!(f, "Material not found"),
            MeetingsError::NotificationFailed => write!(f, "Notification failed"),
            MeetingsError::CalendarSyncFailed => write!(f, "Calendar sync failed"),
        }
    }
}

impl std::error::Error for MeetingsError {}