use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use uuid::Uuid;

use linkwithmentor_auth::Claims;
use linkwithmentor_common::{ApiResponse, AppError};

use crate::{
    models::{
        ContentAnalysisRequest, ContentAnalysisResponse, ModerationActionRequest,
        ModerationActionResponse, ReportRequest, ReportResponse, ImageAnalysisRequest,
        ImageAnalysisResponse, UserSafetyProfile, ModerationAnalytics,
    },
    image_analyzer::ImageAnalyzer,
    AppState,
};

// Content analysis endpoints
pub async fn analyze_content(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<ContentAnalysisRequest>,
) -> Result<Json<ApiResponse<ContentAnalysisResponse>>, AppError> {
    let analysis = state.content_analyzer
        .analyze_content(request)
        .await?;

    Ok(Json(ApiResponse::success(analysis)))
}

pub async fn analyze_image(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<ImageAnalysisRequest>,
) -> Result<Json<ApiResponse<ImageAnalysisResponse>>, AppError> {
    let analysis = ImageAnalyzer::analyze_image(request).await?;
    Ok(Json(ApiResponse::success(analysis)))
}

// Moderation action endpoints
pub async fn execute_moderation_action(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<ModerationActionRequest>,
) -> Result<Json<ApiResponse<ModerationActionResponse>>, AppError> {
    // Check if user has moderation permissions
    if !claims.roles.contains(&"moderator".to_string()) && !claims.roles.contains(&"admin".to_string()) {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    let action = state.moderation_engine
        .execute_moderation_action(request)
        .await?;

    Ok(Json(ApiResponse::success(action)))
}

// Reporting endpoints
pub async fn submit_report(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<ReportRequest>,
) -> Result<Json<ApiResponse<ReportResponse>>, AppError> {
    let report = state.reporting_service
        .submit_report(claims.user_id, request)
        .await?;

    Ok(Json(ApiResponse::success(report)))
}

// User safety endpoints
pub async fn get_user_safety_profile(
    State(state): State<AppState>,
    claims: Claims,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<UserSafetyProfile>>, AppError> {
    // Check permissions - users can only see their own profile, moderators can see any
    if user_id != claims.user_id && !claims.roles.contains(&"moderator".to_string()) {
        return Err(AppError::Forbidden("Cannot access other user's safety profile".to_string()));
    }

    let profile = state.moderation_engine
        .get_user_safety_profile(user_id)
        .await?;

    Ok(Json(ApiResponse::success(profile)))
}

// Analytics endpoints (admin/moderator only)
pub async fn get_moderation_analytics(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<ModerationAnalytics>>, AppError> {
    if !claims.roles.contains(&"moderator".to_string()) && !claims.roles.contains(&"admin".to_string()) {
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    // Placeholder analytics
    let analytics = ModerationAnalytics {
        total_content_analyzed: 10000,
        violations_detected: 150,
        automated_actions: 120,
        manual_reviews: 30,
        appeals_submitted: 5,
        appeals_upheld: 2,
        top_violation_types: Vec::new(),
        moderation_accuracy: 0.95,
        average_response_time_hours: 2.5,
    };

    Ok(Json(ApiResponse::success(analytics)))
}

// Health check endpoint
pub async fn health_check() -> Result<Json<ApiResponse<String>>, AppError> {
    Ok(Json(ApiResponse::success("Safety & Moderation service is healthy".to_string())))
}