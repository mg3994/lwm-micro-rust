use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// WebRTC Signaling Messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalingMessage {
    // Call initiation
    CallOffer {
        call_id: Uuid,
        caller_id: Uuid,
        callee_id: Uuid,
        session_id: Option<Uuid>,
        sdp: String,
        call_type: CallType,
    },
    CallAnswer {
        call_id: Uuid,
        sdp: String,
    },
    CallReject {
        call_id: Uuid,
        reason: String,
    },
    CallCancel {
        call_id: Uuid,
    },
    CallEnd {
        call_id: Uuid,
    },
    
    // ICE candidates
    IceCandidate {
        call_id: Uuid,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u32>,
    },
    
    // Call state changes
    CallStateChanged {
        call_id: Uuid,
        state: CallState,
        participant_id: Uuid,
    },
    
    // Participant management
    ParticipantJoined {
        call_id: Uuid,
        participant_id: Uuid,
        username: String,
    },
    ParticipantLeft {
        call_id: Uuid,
        participant_id: Uuid,
        username: String,
    },
    
    // Media control
    MediaStateChanged {
        call_id: Uuid,
        participant_id: Uuid,
        audio_enabled: bool,
        video_enabled: bool,
        screen_sharing: bool,
    },
    
    // Screen sharing
    ScreenShareOffer {
        call_id: Uuid,
        participant_id: Uuid,
        sdp: String,
    },
    ScreenShareAnswer {
        call_id: Uuid,
        participant_id: Uuid,
        sdp: String,
    },
    ScreenShareEnd {
        call_id: Uuid,
        participant_id: Uuid,
    },
    
    // Call quality
    QualityReport {
        call_id: Uuid,
        participant_id: Uuid,
        metrics: CallQualityMetrics,
    },
    
    // Error handling
    Error {
        call_id: Option<Uuid>,
        code: String,
        message: String,
    },
    
    // Heartbeat
    Ping,
    Pong,
}

// REST API Models
#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateCallRequest {
    pub callee_id: Uuid,
    pub session_id: Option<Uuid>,
    pub call_type: CallType,
    pub sdp_offer: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallResponse {
    pub call_id: Uuid,
    pub caller_id: Uuid,
    pub callee_id: Uuid,
    pub session_id: Option<Uuid>,
    pub call_type: CallType,
    pub state: CallState,
    pub created_at: DateTime<Utc>,
    pub turn_credentials: Option<TurnCredentials>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnswerCallRequest {
    pub sdp_answer: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RejectCallRequest {
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddIceCandidateRequest {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateMediaStateRequest {
    pub audio_enabled: bool,
    pub video_enabled: bool,
    pub screen_sharing: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartScreenShareRequest {
    pub sdp_offer: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenShareResponse {
    pub sdp_answer: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallQualityRequest {
    pub metrics: CallQualityMetrics,
}

// Core Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CallType {
    Audio,
    Video,
    ScreenShare,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CallState {
    Initiating,
    Ringing,
    Connecting,
    Connected,
    OnHold,
    Ended,
    Failed,
    Cancelled,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallParticipant {
    pub user_id: Uuid,
    pub username: String,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub media_state: MediaState,
    pub connection_state: ParticipantConnectionState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaState {
    pub audio_enabled: bool,
    pub video_enabled: bool,
    pub screen_sharing: bool,
    pub audio_muted: bool,
    pub video_muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticipantConnectionState {
    Connecting,
    Connected,
    Disconnected,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallQualityMetrics {
    pub audio_bitrate: Option<u32>,
    pub video_bitrate: Option<u32>,
    pub packet_loss: Option<f32>,
    pub jitter: Option<f32>,
    pub rtt: Option<u32>,
    pub bandwidth: Option<u32>,
    pub resolution: Option<String>,
    pub frame_rate: Option<u32>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnCredentials {
    pub username: String,
    pub password: String,
    pub ttl: u32,
    pub uris: Vec<String>,
}

// Database Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSession {
    pub call_id: Uuid,
    pub caller_id: Uuid,
    pub callee_id: Uuid,
    pub session_id: Option<Uuid>,
    pub call_type: CallType,
    pub state: CallState,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
    pub quality_metrics: Option<serde_json::Value>,
    pub recording_path: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallParticipantDb {
    pub call_id: Uuid,
    pub user_id: Uuid,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub media_state: serde_json::Value,
    pub connection_quality: Option<serde_json::Value>,
}

// Connection Management
#[derive(Debug, Clone)]
pub struct CallConnection {
    pub call_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub connection_id: String,
    pub sender: tokio::sync::mpsc::UnboundedSender<tokio_tungstenite::tungstenite::Message>,
    pub connected_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub media_state: MediaState,
}

#[derive(Debug, Clone)]
pub struct ActiveCall {
    pub call_id: Uuid,
    pub caller_id: Uuid,
    pub callee_id: Uuid,
    pub session_id: Option<Uuid>,
    pub call_type: CallType,
    pub state: CallState,
    pub participants: HashMap<Uuid, CallParticipant>,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub recording_active: bool,
    pub screen_sharing_participant: Option<Uuid>,
}

// Recording Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallRecording {
    pub recording_id: Uuid,
    pub call_id: Uuid,
    pub file_path: String,
    pub file_size: i64,
    pub duration_seconds: i32,
    pub format: String,
    pub quality: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartRecordingRequest {
    pub quality: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordingResponse {
    pub recording_id: Uuid,
    pub status: RecordingStatus,
    pub file_path: Option<String>,
    pub duration_seconds: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordingStatus {
    Starting,
    Recording,
    Stopping,
    Completed,
    Failed,
}

// Analytics Models
#[derive(Debug, Serialize, Deserialize)]
pub struct CallAnalytics {
    pub call_id: Uuid,
    pub total_duration_seconds: i32,
    pub participant_count: i32,
    pub average_quality_score: f32,
    pub connection_issues: i32,
    pub screen_sharing_duration: Option<i32>,
    pub recording_duration: Option<i32>,
    pub bandwidth_usage: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallStatistics {
    pub total_calls: i64,
    pub active_calls: i64,
    pub average_call_duration: f32,
    pub success_rate: f32,
    pub quality_distribution: HashMap<String, i32>,
    pub peak_concurrent_calls: i32,
}

// Error Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallError {
    CallNotFound,
    ParticipantNotFound,
    InvalidCallState,
    MaxParticipantsReached,
    MediaNegotiationFailed,
    ConnectionFailed,
    RecordingFailed,
    PermissionDenied,
    ServiceUnavailable,
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallError::CallNotFound => write!(f, "Call not found"),
            CallError::ParticipantNotFound => write!(f, "Participant not found"),
            CallError::InvalidCallState => write!(f, "Invalid call state"),
            CallError::MaxParticipantsReached => write!(f, "Maximum participants reached"),
            CallError::MediaNegotiationFailed => write!(f, "Media negotiation failed"),
            CallError::ConnectionFailed => write!(f, "Connection failed"),
            CallError::RecordingFailed => write!(f, "Recording failed"),
            CallError::PermissionDenied => write!(f, "Permission denied"),
            CallError::ServiceUnavailable => write!(f, "Service unavailable"),
        }
    }
}

impl std::error::Error for CallError {}