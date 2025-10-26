use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use uuid::Uuid;

use linkwithmentor_auth::Claims;
use linkwithmentor_common::{ApiResponse, AppError};

use crate::{
    models::{
        NotificationRequest, NotificationResponse, NotificationPreferences,
        NotificationStatus,
    },
    AppState,
};

// Send notification
pub async fn send_notification(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<NotificationRequest>,
) -> Result<Json<ApiResponse<NotificationResponse>>, AppError> {
    // Deliver notification
    state.delivery_manager.deliver_notification(&request).await?;

    let notification_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let response = NotificationResponse {
        notification_id,
        recipient_id: request.recipient_id,
        status: NotificationStatus::Sent,
        channels: Vec::new(),
        created_at: now,
        scheduled_at: request.scheduled_at,
        sent_at: Some(now),
    };

    Ok(Json(ApiResponse::success(response)))
}

// Get notification preferences
pub async fn get_preferences(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<NotificationPreferences>>, AppError> {
    let preferences = NotificationPreferences {
        user_id: claims.user_id,
        preferences: std::collections::HashMap::new(),
        quiet_hours_start: None,
        quiet_hours_end: None,
        timezone: "UTC".to_string(),
        updated_at: chrono::Utc::now(),
    };

    Ok(Json(ApiResponse::success(preferences)))
}

// Update notification preferences
pub async fn update_preferences(
    State(state): State<AppState>,
    claims: Claims,
    Json(preferences): Json<NotificationPreferences>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // Implementation to update preferences
    Ok(Json(ApiResponse::success(())))
}

// Health check
pub async fn health_check() -> Result<Json<ApiResponse<String>>, AppError> {
    Ok(Json(ApiResponse::success("Notification service is healthy".to_string())))
}