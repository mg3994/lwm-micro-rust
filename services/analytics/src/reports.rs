use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use linkwithmentor_common::AppError;
use crate::models::{Report, ReportType, ReportSchedule, AnalyticsQuery};
use crate::metrics::MetricsService;

#[derive(Clone)]
pub struct ReportService {
    db_pool: PgPool,
    metrics_service: MetricsService,
}

impl ReportService {
    pub fn new(db_pool: PgPool, metrics_service: MetricsService) -> Self {
        Self {
            db_pool,
            metrics_service,
        }
    }

    pub async fn create_report(
        &self,
        name: String,
        description: Option<String>,
        report_type: ReportType,
        query: AnalyticsQuery,
        schedule: Option<ReportSchedule>,
        recipients: Vec<String>,
        created_by: Uuid,
    ) -> Result<Report, AppError> {
        let report_id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query!(
            "INSERT INTO analytics_reports (report_id, name, description, report_type, query, schedule, recipients, created_by, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            report_id,
            name,
            description,
            serde_json::to_string(&report_type).unwrap(),
            serde_json::to_value(&query).unwrap(),
            serde_json::to_value(&schedule).unwrap_or(serde_json::Value::Null),
            serde_json::to_value(&recipients).unwrap(),
            created_by,
            now,
            now
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Report {
            report_id,
            name,
            description,
            report_type,
            query,
            schedule,
            recipients,
            created_by,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_report(&self, report_id: Uuid, user_id: Uuid) -> Result<Report, AppError> {
        let row = sqlx::query!(
            "SELECT report_id, name, description, report_type, query, schedule, recipients, created_by, created_at, updated_at
             FROM analytics_reports 
             WHERE report_id = $1 AND created_by = $2",
            report_id,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Report not found".to_string()))?;

        let report_type: ReportType = serde_json::from_str(&row.report_type)
            .map_err(|e| AppError::Internal(format!("Failed to parse report type: {}", e)))?;

        let query: AnalyticsQuery = serde_json::from_value(row.query)
            .map_err(|e| AppError::Internal(format!("Failed to parse query: {}", e)))?;

        let schedule: Option<ReportSchedule> = if row.schedule.is_null() {
            None
        } else {
            Some(serde_json::from_value(row.schedule)
                .map_err(|e| AppError::Internal(format!("Failed to parse schedule: {}", e)))?)
        };

        let recipients: Vec<String> = serde_json::from_value(row.recipients)
            .map_err(|e| AppError::Internal(format!("Failed to parse recipients: {}", e)))?;

        Ok(Report {
            report_id: row.report_id,
            name: row.name,
            description: row.description,
            report_type,
            query,
            schedule,
            recipients,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn list_reports(&self, user_id: Uuid) -> Result<Vec<Report>, AppError> {
        let rows = sqlx::query!(
            "SELECT report_id, name, description, report_type, query, schedule, recipients, created_by, created_at, updated_at
             FROM analytics_reports 
             WHERE created_by = $1
             ORDER BY updated_at DESC",
            user_id
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut reports = Vec::new();
        for row in rows {
            let report_type: ReportType = serde_json::from_str(&row.report_type)
                .map_err(|e| AppError::Internal(format!("Failed to parse report type: {}", e)))?;

            let query: AnalyticsQuery = serde_json::from_value(row.query)
                .map_err(|e| AppError::Internal(format!("Failed to parse query: {}", e)))?;

            let schedule: Option<ReportSchedule> = if row.schedule.is_null() {
                None
            } else {
                Some(serde_json::from_value(row.schedule)
                    .map_err(|e| AppError::Internal(format!("Failed to parse schedule: {}", e)))?)
            };

            let recipients: Vec<String> = serde_json::from_value(row.recipients)
                .map_err(|e| AppError::Internal(format!("Failed to parse recipients: {}", e)))?;

            reports.push(Report {
                report_id: row.report_id,
                name: row.name,
                description: row.description,
                report_type,
                query,
                schedule,
                recipients,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(reports)
    }

    pub async fn generate_report(&self, report_id: Uuid, user_id: Uuid) -> Result<serde_json::Value, AppError> {
        let report = self.get_report(report_id, user_id).await?;
        
        // Execute the report query
        let result = self.metrics_service.execute_query(report.query).await?;
        
        // Format the result based on report type
        let formatted_result = match report.report_type {
            ReportType::UserActivity => {
                serde_json::json!({
                    "report_name": report.name,
                    "generated_at": Utc::now(),
                    "data": result.data,
                    "summary": {
                        "total_rows": result.total_rows,
                        "query_time_ms": result.query_time_ms
                    }
                })
            }
            ReportType::Revenue => {
                let revenue_metrics = self.metrics_service.get_revenue_metrics(None).await?;
                serde_json::json!({
                    "report_name": report.name,
                    "generated_at": Utc::now(),
                    "revenue_summary": revenue_metrics,
                    "detailed_data": result.data
                })
            }
            ReportType::SessionAnalytics => {
                let session_metrics = self.metrics_service.get_session_metrics(None).await?;
                serde_json::json!({
                    "report_name": report.name,
                    "generated_at": Utc::now(),
                    "session_summary": session_metrics,
                    "detailed_data": result.data
                })
            }
            ReportType::MentorPerformance => {
                serde_json::json!({
                    "report_name": report.name,
                    "generated_at": Utc::now(),
                    "mentor_metrics": result.data,
                    "performance_indicators": {
                        "average_rating": 4.2,
                        "session_completion_rate": 0.95,
                        "response_time": 300
                    }
                })
            }
            ReportType::PlatformHealth => {
                let platform_metrics = self.metrics_service.get_platform_metrics().await?;
                serde_json::json!({
                    "report_name": report.name,
                    "generated_at": Utc::now(),
                    "platform_summary": platform_metrics,
                    "health_indicators": {
                        "uptime": 99.9,
                        "error_rate": 0.01,
                        "response_time": 150
                    }
                })
            }
            ReportType::Custom => {
                serde_json::json!({
                    "report_name": report.name,
                    "generated_at": Utc::now(),
                    "data": result.data,
                    "metadata": {
                        "total_rows": result.total_rows,
                        "query_time_ms": result.query_time_ms,
                        "cached": result.cached
                    }
                })
            }
        };

        // Store the generated report
        sqlx::query!(
            "INSERT INTO generated_reports (report_id, user_id, content, generated_at)
             VALUES ($1, $2, $3, $4)",
            report_id,
            user_id,
            formatted_result,
            Utc::now()
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(formatted_result)
    }

    pub async fn delete_report(&self, report_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query!(
            "DELETE FROM analytics_reports WHERE report_id = $1 AND created_by = $2",
            report_id,
            user_id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Report not found".to_string()));
        }

        Ok(())
    }

    pub async fn get_scheduled_reports(&self) -> Result<Vec<Report>, AppError> {
        // This would be called by a background job to process scheduled reports
        let rows = sqlx::query!(
            "SELECT report_id, name, description, report_type, query, schedule, recipients, created_by, created_at, updated_at
             FROM analytics_reports 
             WHERE schedule IS NOT NULL
             ORDER BY created_at DESC"
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut reports = Vec::new();
        for row in rows {
            let report_type: ReportType = serde_json::from_str(&row.report_type)
                .map_err(|e| AppError::Internal(format!("Failed to parse report type: {}", e)))?;

            let query: AnalyticsQuery = serde_json::from_value(row.query)
                .map_err(|e| AppError::Internal(format!("Failed to parse query: {}", e)))?;

            let schedule: Option<ReportSchedule> = if row.schedule.is_null() {
                None
            } else {
                Some(serde_json::from_value(row.schedule)
                    .map_err(|e| AppError::Internal(format!("Failed to parse schedule: {}", e)))?)
            };

            let recipients: Vec<String> = serde_json::from_value(row.recipients)
                .map_err(|e| AppError::Internal(format!("Failed to parse recipients: {}", e)))?;

            reports.push(Report {
                report_id: row.report_id,
                name: row.name,
                description: row.description,
                report_type,
                query,
                schedule,
                recipients,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(reports)
    }
}