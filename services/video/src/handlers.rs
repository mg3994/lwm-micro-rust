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
        InitiateCallRequest, CallResponse, AnswerCallRequest, RejectCallRequest,
        AddIceCandidateRequest, UpdateMediaStateRequest, StartScreenShareRequest,
        ScreenShareResponse, CallQualityRequest, CallStatistics, CallAnalytics,
        StartRecordingRequest, RecordingResponse, CallState, CallType,
    },
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct CallQuery {
    pub session_id: Option<Uuid>,
    pub call_type: Option<String>,
}

// Initiate a new call
pub async fn initiate_call(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<InitiateCallRequest>,
) -> Result<Json<ApiResponse<CallResponse>>, AppError> {
    // Validate call type
    let call_type = request.call_type;
    
    // Check if user is already in a call
    if state.call_manager.is_user_in_call(claims.user_id).await {
        return Err(AppError::BadRequest("User is already in a call".to_string()));
    }

    // Check if callee exists (in a real app, verify user exists in database)
    if request.callee_id == claims.user_id {
        return Err(AppError::BadRequest("Cannot call yourself".to_string()));
    }

    // Create the call
    let call_id = state.signaling_service
        .handle_call_offer(
            claims.user_id,
            request.callee_id,
            request.session_id,
            call_type.clone(),
            request.sdp_offer,
        )
        .await?;

    // Generate TURN credentials
    let turn_credentials = state.signaling_service
        .get_turn_credentials(claims.user_id)
        .await
        .ok();

    let response = CallResponse {
        call_id,
        caller_id: claims.user_id,
        callee_id: request.callee_id,
        session_id: request.session_id,
        call_type,
        state: CallState::Initiating,
        created_at: chrono::Utc::now(),
        turn_credentials,
    };

    Ok(Json(ApiResponse::success(response)))
}

// Answer an incoming call
pub async fn answer_call(
    State(state): State<AppState>,
    claims: Claims,
    Path(call_id): Path<Uuid>,
    Json(request): Json<AnswerCallRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.signaling_service
        .handle_call_answer(call_id, claims.user_id, request.sdp_answer)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// Reject an incoming call
pub async fn reject_call(
    State(state): State<AppState>,
    claims: Claims,
    Path(call_id): Path<Uuid>,
    Json(request): Json<RejectCallRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.signaling_service
        .handle_call_reject(call_id, claims.user_id, request.reason)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// End a call
pub async fn end_call(
    State(state): State<AppState>,
    claims: Claims,
    Path(call_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.signaling_service
        .handle_call_end(call_id, claims.user_id)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// Add ICE candidate
pub async fn add_ice_candidate(
    State(state): State<AppState>,
    claims: Claims,
    Path(call_id): Path<Uuid>,
    Json(request): Json<AddIceCandidateRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.signaling_service
        .handle_ice_candidate(
            call_id,
            claims.user_id,
            request.candidate,
            request.sdp_mid,
            request.sdp_mline_index,
        )
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// Update media state (mute/unmute audio/video)
pub async fn update_media_state(
    State(state): State<AppState>,
    claims: Claims,
    Path(call_id): Path<Uuid>,
    Json(request): Json<UpdateMediaStateRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.signaling_service
        .handle_media_state_change(
            call_id,
            claims.user_id,
            request.audio_enabled,
            request.video_enabled,
            request.screen_sharing,
        )
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// Start screen sharing
pub async fn start_screen_share(
    State(state): State<AppState>,
    claims: Claims,
    Path(call_id): Path<Uuid>,
    Json(request): Json<StartScreenShareRequest>,
) -> Result<Json<ApiResponse<ScreenShareResponse>>, AppError> {
    // Start screen sharing in call manager
    state.call_manager
        .start_screen_sharing(call_id, claims.user_id)
        .await?;

    // In a real implementation, you would negotiate the screen share SDP
    // For now, return a placeholder response
    let response = ScreenShareResponse {
        sdp_answer: "placeholder_sdp_answer".to_string(),
    };

    Ok(Json(ApiResponse::success(response)))
}

// Stop screen sharing
pub async fn stop_screen_share(
    State(state): State<AppState>,
    claims: Claims,
    Path(call_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.call_manager
        .stop_screen_sharing(call_id, claims.user_id)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// Submit call quality metrics
pub async fn submit_quality_metrics(
    State(state): State<AppState>,
    claims: Claims,
    Path(call_id): Path<Uuid>,
    Json(request): Json<CallQualityRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.call_manager
        .record_quality_metrics(call_id, claims.user_id, request.metrics)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// Get call information
pub async fn get_call_info(
    State(state): State<AppState>,
    claims: Claims,
    Path(call_id): Path<Uuid>,
) -> Result<Json<ApiResponse<CallResponse>>, AppError> {
    let call = state.call_manager.get_call(call_id).await
        .ok_or_else(|| AppError::NotFound("Call not found".to_string()))?;

    // Verify user is part of the call
    if !call.participants.contains_key(&claims.user_id) {
        return Err(AppError::Forbidden("Not a participant in this call".to_string()));
    }

    let response = CallResponse {
        call_id: call.call_id,
        caller_id: call.caller_id,
        callee_id: call.callee_id,
        session_id: call.session_id,
        call_type: call.call_type,
        state: call.state,
        created_at: call.started_at,
        turn_credentials: None, // Don't include credentials in info response
    };

    Ok(Json(ApiResponse::success(response)))
}

// Get TURN/STUN server credentials
pub async fn get_ice_servers(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<Vec<crate::turn_client::IceServer>>>, AppError> {
    let ice_servers = state.signaling_service
        .get_ice_servers(claims.user_id)
        .await?;

    Ok(Json(ApiResponse::success(ice_servers)))
}

// Start call recording
pub async fn start_recording(
    State(state): State<AppState>,
    claims: Claims,
    Path(call_id): Path<Uuid>,
    Json(request): Json<StartRecordingRequest>,
) -> Result<Json<ApiResponse<RecordingResponse>>, AppError> {
    // Verify user is part of the call
    let call = state.call_manager.get_call(call_id).await
        .ok_or_else(|| AppError::NotFound("Call not found".to_string()))?;

    if !call.participants.contains_key(&claims.user_id) {
        return Err(AppError::Forbidden("Not a participant in this call".to_string()));
    }

    // Check if recording is enabled
    if !state.config.video.enable_call_recording {
        return Err(AppError::BadRequest("Call recording is not enabled".to_string()));
    }

    // In a real implementation, you would start the recording process
    let recording_id = Uuid::new_v4();
    
    let response = RecordingResponse {
        recording_id,
        status: crate::models::RecordingStatus::Starting,
        file_path: None,
        duration_seconds: None,
    };

    tracing::info!("Started recording for call {} by user {}", call_id, claims.user_id);

    Ok(Json(ApiResponse::success(response)))
}

// Stop call recording
pub async fn stop_recording(
    State(state): State<AppState>,
    claims: Claims,
    Path(call_id): Path<Uuid>,
) -> Result<Json<ApiResponse<RecordingResponse>>, AppError> {
    // Verify user is part of the call
    let call = state.call_manager.get_call(call_id).await
        .ok_or_else(|| AppError::NotFound("Call not found".to_string()))?;

    if !call.participants.contains_key(&claims.user_id) {
        return Err(AppError::Forbidden("Not a participant in this call".to_string()));
    }

    // In a real implementation, you would stop the recording process
    let recording_id = Uuid::new_v4(); // This would be the actual recording ID
    
    let response = RecordingResponse {
        recording_id,
        status: crate::models::RecordingStatus::Stopping,
        file_path: Some("/recordings/call_recording.mp4".to_string()),
        duration_seconds: Some(300), // 5 minutes example
    };

    tracing::info!("Stopped recording for call {} by user {}", call_id, claims.user_id);

    Ok(Json(ApiResponse::success(response)))
}

// Get call analytics
pub async fn get_call_analytics(
    State(state): State<AppState>,
    claims: Claims,
    Path(call_id): Path<Uuid>,
) -> Result<Json<ApiResponse<CallAnalytics>>, AppError> {
    let call = state.call_manager.get_call(call_id).await
        .ok_or_else(|| AppError::NotFound("Call not found".to_string()))?;

    // Verify user is part of the call or has admin access
    if !call.participants.contains_key(&claims.user_id) && !claims.roles.contains(&"admin".to_string()) {
        return Err(AppError::Forbidden("Not authorized to view call analytics".to_string()));
    }

    // Calculate analytics (in a real implementation, this would query the database)
    let analytics = CallAnalytics {
        call_id,
        total_duration_seconds: (chrono::Utc::now() - call.started_at).num_seconds() as i32,
        participant_count: call.participants.len() as i32,
        average_quality_score: 4.2, // Placeholder
        connection_issues: 0,
        screen_sharing_duration: None,
        recording_duration: None,
        bandwidth_usage: Some(1024 * 1024), // 1MB placeholder
    };

    Ok(Json(ApiResponse::success(analytics)))
}

// Get platform call statistics (admin only)
pub async fn get_call_statistics(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<CallStatistics>>, AppError> {
    // Check admin access
    if !claims.roles.contains(&"admin".to_string()) {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    // In a real implementation, this would query the database for statistics
    let statistics = CallStatistics {
        total_calls: 1000,
        active_calls: 5,
        average_call_duration: 15.5, // minutes
        success_rate: 0.95,
        quality_distribution: std::collections::HashMap::from([
            ("excellent".to_string(), 60),
            ("good".to_string(), 30),
            ("fair".to_string(), 8),
            ("poor".to_string(), 2),
        ]),
        peak_concurrent_calls: 25,
    };

    Ok(Json(ApiResponse::success(statistics)))
}

// Health check endpoint
pub async fn health_check() -> Result<Json<ApiResponse<String>>, AppError> {
    Ok(Json(ApiResponse::success("Video service is healthy".to_string())))
}

// Get active calls for a user
pub async fn get_user_calls(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<Vec<CallResponse>>>, AppError> {
    // In a real implementation, this would query active calls for the user
    let mut user_calls = Vec::new();

    // Check if user is currently in any calls
    if state.call_manager.is_user_in_call(claims.user_id).await {
        // Find and return the user's active calls
        // This is a simplified implementation
        tracing::debug!("User {} has active calls", claims.user_id);
    }

    Ok(Json(ApiResponse::success(user_calls)))
}