use uuid::Uuid;
use sqlx::PgPool;
use chrono::Utc;

use linkwithmentor_common::{AppError, RedisService};
use crate::{
    models::{
        ReportRequest, ReportResponse, ReportStatus, ReportPriority,
        InvestigationRequest, InvestigationResponse, InvestigationStatus,
    },
    moderation_engine::ModerationEngine,
};

#[derive(Clone)]
pub struct ReportingService {
    db_pool: PgPool,
    redis_service: RedisService,
    moderation_engine: ModerationEngine,
}

impl ReportingService {
    pub fn new(
        db_pool: PgPool,
        redis_service: RedisService,
        moderation_engine: ModerationEngine,
    ) -> Self {
        Self {
            db_pool,
            redis_service,
            moderation_engine,
        }
    }

    pub async fn initialize(&self) -> Result<(), AppError> {
        tracing::info!("Reporting service initialized");
        Ok(())
    }

    pub async fn submit_report(
        &self,
        reporter_id: Uuid,
        request: ReportRequest,
    ) -> Result<ReportResponse, AppError> {
        let report_id = Uuid::new_v4();
        let now = Utc::now();

        // Determine priority based on report type
        let priority = match request.report_type {
            crate::models::ReportType::Violence | 
            crate::models::ReportType::HateSpeech => ReportPriority::High,
            crate::models::ReportType::Harassment => ReportPriority::Medium,
            _ => ReportPriority::Low,
        };

        // Store report
        let query = r#"
            INSERT INTO reports (
                report_id, reporter_id, reported_user_id, reported_content_type,
                reported_content_id, report_type, status, priority,
                description, evidence, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#;

        let evidence_json = serde_json::to_value(&request.evidence)
            .map_err(|e| AppError::Internal(format!("Failed to serialize evidence: {}", e)))?;

        sqlx::query(query)
            .bind(report_id)
            .bind(reporter_id)
            .bind(request.reported_user_id)
            .bind(&request.reported_content_type)
            .bind(request.reported_content_id)
            .bind(&request.report_type)
            .bind(&ReportStatus::Submitted)
            .bind(&priority)
            .bind(&request.description)
            .bind(evidence_json)
            .bind(now)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to store report: {}", e)))?;

        Ok(ReportResponse {
            report_id,
            reporter_id,
            reported_user_id: request.reported_user_id,
            report_type: request.report_type,
            status: ReportStatus::Submitted,
            priority,
            assigned_moderator: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn start_investigation(
        &self,
        request: InvestigationRequest,
    ) -> Result<InvestigationResponse, AppError> {
        let investigation_id = Uuid::new_v4();
        let now = Utc::now();

        Ok(InvestigationResponse {
            investigation_id,
            report_id: request.report_id,
            moderator_id: request.moderator_id,
            status: InvestigationStatus::InProgress,
            findings: request.investigation_notes,
            evidence: Vec::new(),
            recommended_action: request.recommended_action,
            created_at: now,
            completed_at: None,
        })
    }
}