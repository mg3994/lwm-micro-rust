use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// Video Lecture Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoLecture {
    pub lecture_id: Uuid,
    pub mentor_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
    pub duration_seconds: Option<u32>,
    pub thumbnail_url: Option<String>,
    pub video_urls: HashMap<String, String>, // quality -> url
    pub status: VideoStatus,
    pub visibility: VideoVisibility,
    pub price: Option<rust_decimal::Decimal>,
    pub view_count: u64,
    pub like_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLectureRequest {
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
    pub visibility: VideoVisibility,
    pub price: Option<rust_decimal::Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLectureRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub visibility: Option<VideoVisibility>,
    pub price: Option<rust_decimal::Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoUploadRequest {
    pub lecture_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoUploadResponse {
    pub upload_id: Uuid,
    pub upload_url: String,
    pub expires_at: DateTime<Utc>,
    pub chunk_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProcessingJob {
    pub job_id: Uuid,
    pub lecture_id: Uuid,
    pub input_file_path: String,
    pub status: ProcessingStatus,
    pub progress_percentage: u8,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub processing_time_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoAnalytics {
    pub lecture_id: Uuid,
    pub total_views: u64,
    pub unique_viewers: u64,
    pub average_watch_time_seconds: f64,
    pub completion_rate: f64,
    pub engagement_score: f64,
    pub views_by_day: Vec<DailyViews>,
    pub viewer_demographics: ViewerDemographics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyViews {
    pub date: chrono::NaiveDate,
    pub views: u64,
    pub unique_viewers: u64,
    pub watch_time_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerDemographics {
    pub age_groups: HashMap<String, u64>,
    pub countries: HashMap<String, u64>,
    pub devices: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchProgress {
    pub user_id: Uuid,
    pub lecture_id: Uuid,
    pub progress_seconds: u32,
    pub completion_percentage: f64,
    pub last_watched_at: DateTime<Utc>,
    pub is_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoComment {
    pub comment_id: Uuid,
    pub lecture_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub content: String,
    pub timestamp_seconds: Option<u32>, // For time-based comments
    pub parent_comment_id: Option<Uuid>, // For replies
    pub like_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub timestamp_seconds: Option<u32>,
    pub parent_comment_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoPlaylist {
    pub playlist_id: Uuid,
    pub creator_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub visibility: PlaylistVisibility,
    pub lecture_count: u32,
    pub total_duration_seconds: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub playlist_id: Uuid,
    pub lecture_id: Uuid,
    pub position: u32,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlaylistRequest {
    pub title: String,
    pub description: Option<String>,
    pub visibility: PlaylistVisibility,
    pub lecture_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTranscript {
    pub transcript_id: Uuid,
    pub lecture_id: Uuid,
    pub language: String,
    pub content: String,
    pub segments: Vec<TranscriptSegment>,
    pub is_auto_generated: bool,
    pub accuracy_score: Option<f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start_time_seconds: f64,
    pub end_time_seconds: f64,
    pub text: String,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSubtitle {
    pub subtitle_id: Uuid,
    pub lecture_id: Uuid,
    pub language: String,
    pub file_url: String,
    pub format: SubtitleFormat,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

// Enums
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VideoStatus {
    Uploading,
    Processing,
    Ready,
    Failed,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VideoVisibility {
    Public,
    Unlisted,
    Private,
    Premium, // Requires payment
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessingStatus {
    Queued,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlaylistVisibility {
    Public,
    Unlisted,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubtitleFormat {
    SRT,
    VTT,
    ASS,
}

// Database Models
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VideoLectureDb {
    pub lecture_id: Uuid,
    pub mentor_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
    pub duration_seconds: Option<i32>,
    pub thumbnail_url: Option<String>,
    pub video_urls: Option<serde_json::Value>,
    pub status: String,
    pub visibility: String,
    pub price: Option<rust_decimal::Decimal>,
    pub view_count: i64,
    pub like_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VideoProcessingJobDb {
    pub job_id: Uuid,
    pub lecture_id: Uuid,
    pub input_file_path: String,
    pub status: String,
    pub progress_percentage: i16,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub processing_time_seconds: Option<i32>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WatchProgressDb {
    pub user_id: Uuid,
    pub lecture_id: Uuid,
    pub progress_seconds: i32,
    pub completion_percentage: rust_decimal::Decimal,
    pub last_watched_at: DateTime<Utc>,
    pub is_completed: bool,
}

// Search and Discovery Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSearchRequest {
    pub query: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub mentor_id: Option<Uuid>,
    pub min_duration: Option<u32>,
    pub max_duration: Option<u32>,
    pub sort_by: Option<VideoSortBy>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSearchResponse {
    pub lectures: Vec<VideoLecture>,
    pub total_count: u64,
    pub page: u32,
    pub limit: u32,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VideoSortBy {
    Newest,
    Oldest,
    MostViewed,
    MostLiked,
    Duration,
    Relevance,
}

// Streaming Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingManifest {
    pub lecture_id: Uuid,
    pub format: StreamingFormat,
    pub manifest_url: String,
    pub qualities: Vec<StreamingQuality>,
    pub subtitles: Vec<SubtitleTrack>,
    pub thumbnails: Vec<ThumbnailSprite>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingQuality {
    pub name: String,
    pub resolution: String,
    pub bitrate: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleTrack {
    pub language: String,
    pub label: String,
    pub url: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailSprite {
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub columns: u32,
    pub rows: u32,
    pub interval_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StreamingFormat {
    HLS,
    DASH,
    Progressive,
}

// Error Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoError {
    UploadFailed(String),
    ProcessingFailed(String),
    InvalidFormat,
    FileTooLarge,
    StorageError(String),
    TranscodingError(String),
    NotFound,
    AccessDenied,
    QuotaExceeded,
}

impl std::fmt::Display for VideoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoError::UploadFailed(msg) => write!(f, "Upload failed: {}", msg),
            VideoError::ProcessingFailed(msg) => write!(f, "Processing failed: {}", msg),
            VideoError::InvalidFormat => write!(f, "Invalid video format"),
            VideoError::FileTooLarge => write!(f, "File size exceeds limit"),
            VideoError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            VideoError::TranscodingError(msg) => write!(f, "Transcoding error: {}", msg),
            VideoError::NotFound => write!(f, "Video not found"),
            VideoError::AccessDenied => write!(f, "Access denied"),
            VideoError::QuotaExceeded => write!(f, "Storage quota exceeded"),
        }
    }
}

impl std::error::Error for VideoError {}

// Recommendation Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRecommendation {
    pub lecture_id: Uuid,
    pub score: f64,
    pub reason: RecommendationReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationReason {
    SimilarContent,
    SameMentor,
    PopularInCategory,
    ContinueWatching,
    BasedOnHistory,
}

// Monetization Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoPurchase {
    pub purchase_id: Uuid,
    pub user_id: Uuid,
    pub lecture_id: Uuid,
    pub amount: rust_decimal::Decimal,
    pub currency: String,
    pub payment_id: Uuid,
    pub access_expires_at: Option<DateTime<Utc>>,
    pub purchased_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRevenue {
    pub lecture_id: Uuid,
    pub total_revenue: rust_decimal::Decimal,
    pub total_purchases: u64,
    pub mentor_earnings: rust_decimal::Decimal,
    pub platform_fees: rust_decimal::Decimal,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}