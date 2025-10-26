use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use linkwithmentor_auth::Claims;
use linkwithmentor_common::{ApiResponse, AppError};

use crate::{
    models::{
        UserMetrics, SessionMetrics, RevenueMetrics, PlatformMetrics, EngagementMetrics,
        Dashboard, DashboardWidget, Report, ReportType, AnalyticsQuery, EventTracking
    },
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub start_date: Option<chrono::DateTime<chrono::Utc>>,
    pub end_date: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDashboardRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateReportRequest {
    pub name: String,
    pub description: Option<String>,
    pub report_type: ReportType,
    pub query: AnalyticsQuery,
    pub recipients: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsOverview {
    pub user_metrics: UserMetrics,
    pub session_metrics: SessionMetrics,
    pub revenue_metrics: RevenueMetrics,
    pub platform_metrics: PlatformMetrics,
    pub engagement_metrics: EngagementMetrics,
}

// Get analytics overview
pub async fn get_analytics_overview(
    State(state): State<AppState>,
    claims: Claims,
    Query(params): Query<MetricsQuery>,
) -> Result<Json<ApiResponse<AnalyticsOverview>>, AppError> {
    // Check if user has analytics access
    if !claims.roles.contains(&"admin".to_string()) && !claims.roles.contains(&"analyst".to_string()) {
        return Err(AppError::Forbidden("Analytics access required".to_string()));
    }

    let date_range = params.start_date.zip(params.end_date);

    let user_metrics = state.metrics_service.get_user_metrics(date_range).await?;
    let session_metrics = state.metrics_service.get_session_metrics(date_range).await?;
    let revenue_metrics = state.metrics_service.get_revenue_metrics(date_range).await?;
    let platform_metrics = state.metrics_service.get_platform_metrics().await?;
    let engagement_metrics = state.metrics_service.get_engagement_metrics().await?;

    let overview = AnalyticsOverview {
        user_metrics,
        session_metrics,
        revenue_metrics,
        platform_metrics,
        engagement_metrics,
    };

    Ok(Json(ApiResponse::success(overview)))
}

// Get user metrics
pub async fn get_user_metrics(
    State(state): State<AppState>,
    claims: Claims,
    Query(params): Query<MetricsQuery>,
) -> Result<Json<ApiResponse<UserMetrics>>, AppError> {
    if !claims.roles.contains(&"admin".to_string()) && !claims.roles.contains(&"analyst".to_string()) {
        return Err(AppError::Forbidden("Analytics access required".to_string()));
    }

    let date_range = params.start_date.zip(params.end_date);
    let metrics = state.metrics_service.get_user_metrics(date_range).await?;

    Ok(Json(ApiResponse::success(metrics)))
}

// Get session metrics
pub async fn get_session_metrics(
    State(state): State<AppState>,
    claims: Claims,
    Query(params): Query<MetricsQuery>,
) -> Result<Json<ApiResponse<SessionMetrics>>, AppError> {
    if !claims.roles.contains(&"admin".to_string()) && !claims.roles.contains(&"analyst".to_string()) {
        return Err(AppError::Forbidden("Analytics access required".to_string()));
    }

    let date_range = params.start_date.zip(params.end_date);
    let metrics = state.metrics_service.get_session_metrics(date_range).await?;

    Ok(Json(ApiResponse::success(metrics)))
}

// Get revenue metrics
pub async fn get_revenue_metrics(
    State(state): State<AppState>,
    claims: Claims,
    Query(params): Query<MetricsQuery>,
) -> Result<Json<ApiResponse<RevenueMetrics>>, AppError> {
    if !claims.roles.contains(&"admin".to_string()) && !claims.roles.contains(&"analyst".to_string()) {
        return Err(AppError::Forbidden("Analytics access required".to_string()));
    }

    let date_range = params.start_date.zip(params.end_date);
    let metrics = state.metrics_service.get_revenue_metrics(date_range).await?;

    Ok(Json(ApiResponse::success(metrics)))
}

// Track event
pub async fn track_event(
    State(state): State<AppState>,
    claims: Claims,
    Json(mut event): Json<EventTracking>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // Set the user_id from the authenticated user
    event.user_id = Some(claims.user_id);
    event.timestamp = chrono::Utc::now();

    state.metrics_service.track_event(event).await?;

    Ok(Json(ApiResponse::success(())))
}

// Execute custom query
pub async fn execute_query(
    State(state): State<AppState>,
    claims: Claims,
    Json(query): Json<AnalyticsQuery>,
) -> Result<Json<ApiResponse<crate::models::AnalyticsResult>>, AppError> {
    if !claims.roles.contains(&"admin".to_string()) && !claims.roles.contains(&"analyst".to_string()) {
        return Err(AppError::Forbidden("Analytics access required".to_string()));
    }

    let result = state.metrics_service.execute_query(query).await?;

    Ok(Json(ApiResponse::success(result)))
}

// Dashboard endpoints
pub async fn create_dashboard(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<CreateDashboardRequest>,
) -> Result<Json<ApiResponse<Dashboard>>, AppError> {
    let dashboard = state.dashboard_service.create_dashboard(
        claims.user_id,
        request.name,
        request.description,
    ).await?;

    Ok(Json(ApiResponse::success(dashboard)))
}

pub async fn get_dashboard(
    State(state): State<AppState>,
    claims: Claims,
    Path(dashboard_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Dashboard>>, AppError> {
    let dashboard = state.dashboard_service.get_dashboard(dashboard_id, claims.user_id).await?;

    Ok(Json(ApiResponse::success(dashboard)))
}

pub async fn list_dashboards(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<Vec<Dashboard>>>, AppError> {
    let dashboards = state.dashboard_service.list_dashboards(claims.user_id).await?;

    Ok(Json(ApiResponse::success(dashboards)))
}

pub async fn add_widget(
    State(state): State<AppState>,
    claims: Claims,
    Path(dashboard_id): Path<Uuid>,
    Json(widget): Json<DashboardWidget>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.dashboard_service.add_widget(dashboard_id, claims.user_id, widget).await?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn remove_widget(
    State(state): State<AppState>,
    claims: Claims,
    Path((dashboard_id, widget_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.dashboard_service.remove_widget(dashboard_id, claims.user_id, widget_id).await?;

    Ok(Json(ApiResponse::success(())))
}

// Report endpoints
pub async fn create_report(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<CreateReportRequest>,
) -> Result<Json<ApiResponse<Report>>, AppError> {
    let report = state.report_service.create_report(
        request.name,
        request.description,
        request.report_type,
        request.query,
        None, // No schedule for now
        request.recipients,
        claims.user_id,
    ).await?;

    Ok(Json(ApiResponse::success(report)))
}

pub async fn get_report(
    State(state): State<AppState>,
    claims: Claims,
    Path(report_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Report>>, AppError> {
    let report = state.report_service.get_report(report_id, claims.user_id).await?;

    Ok(Json(ApiResponse::success(report)))
}

pub async fn list_reports(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<Vec<Report>>>, AppError> {
    let reports = state.report_service.list_reports(claims.user_id).await?;

    Ok(Json(ApiResponse::success(reports)))
}

pub async fn generate_report(
    State(state): State<AppState>,
    claims: Claims,
    Path(report_id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = state.report_service.generate_report(report_id, claims.user_id).await?;

    Ok(Json(ApiResponse::success(result)))
}

pub async fn delete_report(
    State(state): State<AppState>,
    claims: Claims,
    Path(report_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.report_service.delete_report(report_id, claims.user_id).await?;

    Ok(Json(ApiResponse::success(())))
}

// Health check
pub async fn health_check() -> Result<Json<ApiResponse<String>>, AppError> {
    Ok(Json(ApiResponse::success("Analytics service is healthy".to_string())))
}