use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use uuid::Uuid;

use linkwithmentor_auth::Claims;
use linkwithmentor_common::{ApiResponse, AppError};

use crate::{
    models::{
        CreateLectureRequest, UpdateLectureRequest, VideoUploadRequest, VideoUploadResponse,
        VideoLecture, VideoSearchRequest, VideoSearchResponse, StreamingManifest, StreamingFormat,
        VideoAnalytics, WatchProgress, CreateCommentRequest, VideoComment,
    },
    AppState,
};

// Lecture management endpoints
pub async fn create_lecture(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<CreateLectureRequest>,
) -> Result<Json<ApiResponse<VideoLecture>>, AppError> {
    let lecture_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // Create lecture in database
    let query = r#"
        INSERT INTO video_lectures (
            lecture_id, mentor_id, title, description, category, tags,
            visibility, price, status, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
    "#;

    sqlx::query(query)
        .bind(lecture_id)
        .bind(claims.user_id)
        .bind(&request.title)
        .bind(&request.description)
        .bind(&request.category)
        .bind(&request.tags)
        .bind(&request.visibility)
        .bind(&request.price)
        .bind(&crate::models::VideoStatus::Uploading)
        .bind(now)
        .bind(now)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create lecture: {}", e)))?;

    let lecture = VideoLecture {
        lecture_id,
        mentor_id: claims.user_id,
        title: request.title,
        description: request.description,
        category: request.category,
        tags: request.tags,
        duration_seconds: None,
        thumbnail_url: None,
        video_urls: std::collections::HashMap::new(),
        status: crate::models::VideoStatus::Uploading,
        visibility: request.visibility,
        price: request.price,
        view_count: 0,
        like_count: 0,
        created_at: now,
        updated_at: now,
        published_at: None,
    };

    Ok(Json(ApiResponse::success(lecture)))
}

pub async fn get_lecture(
    State(state): State<AppState>,
    claims: Claims,
    Path(lecture_id): Path<Uuid>,
) -> Result<Json<ApiResponse<VideoLecture>>, AppError> {
    // Simplified lecture retrieval
    let lecture = VideoLecture {
        lecture_id,
        mentor_id: Uuid::new_v4(),
        title: "Sample Lecture".to_string(),
        description: Some("A sample video lecture".to_string()),
        category: "Programming".to_string(),
        tags: vec!["rust".to_string(), "programming".to_string()],
        duration_seconds: Some(1800),
        thumbnail_url: Some("https://example.com/thumbnail.jpg".to_string()),
        video_urls: std::collections::HashMap::new(),
        status: crate::models::VideoStatus::Ready,
        visibility: crate::models::VideoVisibility::Public,
        price: None,
        view_count: 100,
        like_count: 15,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        published_at: Some(chrono::Utc::now()),
    };

    Ok(Json(ApiResponse::success(lecture)))
}

pub async fn update_lecture(
    State(state): State<AppState>,
    claims: Claims,
    Path(lecture_id): Path<Uuid>,
    Json(request): Json<UpdateLectureRequest>,
) -> Result<Json<ApiResponse<VideoLecture>>, AppError> {
    // Update lecture implementation
    let lecture = VideoLecture {
        lecture_id,
        mentor_id: claims.user_id,
        title: request.title.unwrap_or_else(|| "Updated Lecture".to_string()),
        description: request.description,
        category: request.category.unwrap_or_else(|| "Programming".to_string()),
        tags: request.tags.unwrap_or_default(),
        duration_seconds: Some(1800),
        thumbnail_url: None,
        video_urls: std::collections::HashMap::new(),
        status: crate::models::VideoStatus::Ready,
        visibility: request.visibility.unwrap_or(crate::models::VideoVisibility::Public),
        price: request.price,
        view_count: 100,
        like_count: 15,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        published_at: Some(chrono::Utc::now()),
    };

    Ok(Json(ApiResponse::success(lecture)))
}

// Upload endpoints
pub async fn initiate_upload(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<VideoUploadRequest>,
) -> Result<Json<ApiResponse<VideoUploadResponse>>, AppError> {
    let upload_response = state.upload_service
        .initiate_upload(claims.user_id, request)
        .await?;

    Ok(Json(ApiResponse::success(upload_response)))
}

pub async fn complete_upload(
    State(state): State<AppState>,
    claims: Claims,
    Path((upload_id, lecture_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.upload_service
        .complete_upload(upload_id, lecture_id)
        .await?;

    // Queue processing job
    let input_path = format!("videos/{}/original", lecture_id);
    state.processing_service
        .queue_processing_job(lecture_id, input_path)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// Search and discovery endpoints
pub async fn search_lectures(
    State(state): State<AppState>,
    Query(request): Query<VideoSearchRequest>,
) -> Result<Json<ApiResponse<VideoSearchResponse>>, AppError> {
    // Simplified search implementation
    let lectures = vec![
        VideoLecture {
            lecture_id: Uuid::new_v4(),
            mentor_id: Uuid::new_v4(),
            title: "Introduction to Rust".to_string(),
            description: Some("Learn Rust programming basics".to_string()),
            category: "Programming".to_string(),
            tags: vec!["rust".to_string(), "programming".to_string()],
            duration_seconds: Some(1800),
            thumbnail_url: Some("https://example.com/thumbnail1.jpg".to_string()),
            video_urls: std::collections::HashMap::new(),
            status: crate::models::VideoStatus::Ready,
            visibility: crate::models::VideoVisibility::Public,
            price: None,
            view_count: 150,
            like_count: 25,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            published_at: Some(chrono::Utc::now()),
        },
    ];

    let response = VideoSearchResponse {
        lectures,
        total_count: 1,
        page: request.page.unwrap_or(1),
        limit: request.limit.unwrap_or(20),
        has_more: false,
    };

    Ok(Json(ApiResponse::success(response)))
}

// Streaming endpoints
pub async fn get_streaming_manifest(
    State(state): State<AppState>,
    claims: Claims,
    Path(lecture_id): Path<Uuid>,
    Query(format): Query<StreamingFormatQuery>,
) -> Result<Json<ApiResponse<StreamingManifest>>, AppError> {
    let streaming_format = format.format.unwrap_or(StreamingFormat::HLS);
    
    let manifest = state.streaming_service
        .get_streaming_manifest(lecture_id, streaming_format)
        .await?;

    Ok(Json(ApiResponse::success(manifest)))
}

// Analytics endpoints
pub async fn get_lecture_analytics(
    State(state): State<AppState>,
    claims: Claims,
    Path(lecture_id): Path<Uuid>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<ApiResponse<VideoAnalytics>>, AppError> {
    let analytics = state.analytics_service
        .get_lecture_analytics(lecture_id, params.days.unwrap_or(30))
        .await?;

    Ok(Json(ApiResponse::success(analytics)))
}

pub async fn record_view(
    State(state): State<AppState>,
    claims: Claims,
    Path(lecture_id): Path<Uuid>,
    Json(request): Json<ViewRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.analytics_service
        .record_view(claims.user_id, lecture_id, request.watch_time_seconds)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn update_watch_progress(
    State(state): State<AppState>,
    claims: Claims,
    Path(lecture_id): Path<Uuid>,
    Json(request): Json<ProgressRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.analytics_service
        .update_watch_progress(
            claims.user_id,
            lecture_id,
            request.progress_seconds,
            request.total_duration_seconds,
        )
        .await?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn get_watch_progress(
    State(state): State<AppState>,
    claims: Claims,
    Path(lecture_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Option<WatchProgress>>>, AppError> {
    let progress = state.analytics_service
        .get_user_watch_progress(claims.user_id, lecture_id)
        .await?;

    Ok(Json(ApiResponse::success(progress)))
}

// Comment endpoints
pub async fn create_comment(
    State(state): State<AppState>,
    claims: Claims,
    Path(lecture_id): Path<Uuid>,
    Json(request): Json<CreateCommentRequest>,
) -> Result<Json<ApiResponse<VideoComment>>, AppError> {
    let comment_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let comment = VideoComment {
        comment_id,
        lecture_id,
        user_id: claims.user_id,
        username: claims.username,
        content: request.content,
        timestamp_seconds: request.timestamp_seconds,
        parent_comment_id: request.parent_comment_id,
        like_count: 0,
        created_at: now,
        updated_at: now,
    };

    Ok(Json(ApiResponse::success(comment)))
}

// Health check endpoint
pub async fn health_check() -> Result<Json<ApiResponse<String>>, AppError> {
    Ok(Json(ApiResponse::success("Video Lectures service is healthy".to_string())))
}

// Helper structs for query parameters
#[derive(Debug, Deserialize)]
pub struct StreamingFormatQuery {
    pub format: Option<StreamingFormat>,
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub days: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ViewRequest {
    pub watch_time_seconds: u32,
}

#[derive(Debug, Deserialize)]
pub struct ProgressRequest {
    pub progress_seconds: u32,
    pub total_duration_seconds: u32,
}