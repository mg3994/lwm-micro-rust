use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// Analytics Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMetrics {
    pub total_users: i64,
    pub active_users_daily: i64,
    pub active_users_weekly: i64,
    pub active_users_monthly: i64,
    pub new_users_today: i64,
    pub new_users_this_week: i64,
    pub new_users_this_month: i64,
    pub user_retention_rate: f64,
    pub average_session_duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub total_sessions: i64,
    pub completed_sessions: i64,
    pub cancelled_sessions: i64,
    pub average_session_duration: f64,
    pub total_session_revenue: f64,
    pub sessions_by_category: HashMap<String, i64>,
    pub peak_hours: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueMetrics {
    pub total_revenue: f64,
    pub revenue_today: f64,
    pub revenue_this_week: f64,
    pub revenue_this_month: f64,
    pub average_transaction_value: f64,
    pub revenue_by_category: HashMap<String, f64>,
    pub top_earning_mentors: Vec<MentorRevenue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentorRevenue {
    pub mentor_id: Uuid,
    pub mentor_name: String,
    pub total_revenue: f64,
    pub session_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMetrics {
    pub total_mentors: i64,
    pub active_mentors: i64,
    pub total_mentees: i64,
    pub active_mentees: i64,
    pub mentor_approval_rate: f64,
    pub average_mentor_rating: f64,
    pub total_reviews: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementMetrics {
    pub total_messages: i64,
    pub messages_today: i64,
    pub average_response_time: f64,
    pub video_call_duration: f64,
    pub content_views: i64,
    pub user_interactions: HashMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub dashboard_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub user_id: Uuid,
    pub widgets: Vec<DashboardWidget>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardWidget {
    pub widget_id: Uuid,
    pub widget_type: WidgetType,
    pub title: String,
    pub configuration: serde_json::Value,
    pub position: WidgetPosition,
    pub size: WidgetSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetSize {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WidgetType {
    LineChart,
    BarChart,
    PieChart,
    MetricCard,
    Table,
    Heatmap,
    Gauge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsQuery {
    pub metric: String,
    pub dimensions: Vec<String>,
    pub filters: HashMap<String, serde_json::Value>,
    pub date_range: DateRange,
    pub aggregation: AggregationType,
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationType {
    Sum,
    Count,
    Average,
    Min,
    Max,
    Distinct,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResult {
    pub data: Vec<HashMap<String, serde_json::Value>>,
    pub total_rows: i64,
    pub query_time_ms: i64,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub report_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub report_type: ReportType,
    pub query: AnalyticsQuery,
    pub schedule: Option<ReportSchedule>,
    pub recipients: Vec<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportType {
    UserActivity,
    Revenue,
    SessionAnalytics,
    MentorPerformance,
    PlatformHealth,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSchedule {
    pub frequency: ScheduleFrequency,
    pub time: String, // HH:MM format
    pub timezone: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleFrequency {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTracking {
    pub event_id: Uuid,
    pub user_id: Option<Uuid>,
    pub session_id: Option<String>,
    pub event_name: String,
    pub event_category: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Funnel {
    pub funnel_id: Uuid,
    pub name: String,
    pub steps: Vec<FunnelStep>,
    pub conversion_rates: Vec<f64>,
    pub total_users: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelStep {
    pub step_name: String,
    pub event_name: String,
    pub users_count: i64,
    pub conversion_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cohort {
    pub cohort_id: Uuid,
    pub name: String,
    pub definition: CohortDefinition,
    pub user_count: i64,
    pub retention_data: Vec<CohortRetention>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortDefinition {
    pub criteria: HashMap<String, serde_json::Value>,
    pub time_period: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortRetention {
    pub period: i32,
    pub users_retained: i64,
    pub retention_rate: f64,
}