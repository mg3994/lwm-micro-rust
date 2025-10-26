use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration, NaiveTime, Datelike, Weekday};
use sqlx::PgPool;
use tokio_cron_scheduler::{JobScheduler, Job};

use linkwithmentor_common::{AppError, RedisService};
use crate::{
    models::{
        SessionRequest, SessionResponse, SessionStatus, SessionType, RecurringPattern,
        RecurrenceFrequency, AvailabilityRequest, AvailabilityResponse, AvailabilitySlot,
        SessionParticipant, ParticipantRole, ParticipantStatus, RecurringSeriesResponse,
        UpdateSessionRequest, SessionDb, MeetingsError,
    },
    notifications::NotificationService,
    calendar::CalendarService,
};

#[derive(Clone)]
pub struct SchedulingService {
    db_pool: PgPool,
    redis_service: RedisService,
    notification_service: NotificationService,
    calendar_service: CalendarService,
    scheduler: Option<JobScheduler>,
}

impl SchedulingService {
    pub fn new(
        db_pool: PgPool,
        redis_service: RedisService,
        notification_service: NotificationService,
        calendar_service: CalendarService,
    ) -> Self {
        Self {
            db_pool,
            redis_service,
            notification_service,
            calendar_service,
            scheduler: None,
        }
    }

    pub async fn initialize(&self) -> Result<(), AppError> {
        // Initialize job scheduler for reminders and recurring sessions
        let scheduler = JobScheduler::new().await
            .map_err(|e| AppError::Internal(format!("Failed to create scheduler: {}", e)))?;

        // Start the scheduler
        scheduler.start().await
            .map_err(|e| AppError::Internal(format!("Failed to start scheduler: {}", e)))?;

        // Schedule reminder jobs
        self.schedule_reminder_jobs(&scheduler).await?;
        
        // Schedule recurring session creation
        self.schedule_recurring_session_jobs(&scheduler).await?;

        tracing::info!("Scheduling service initialized");
        Ok(())
    }

    // Session Management
    pub async fn create_session(
        &self,
        mentee_id: Uuid,
        request: SessionRequest,
    ) -> Result<SessionResponse, AppError> {
        // Validate session request
        self.validate_session_request(&request).await?;

        // Check for scheduling conflicts
        self.check_scheduling_conflicts(&request).await?;

        // Check mentor availability
        self.check_mentor_availability(request.mentor_id, &request.scheduled_start, request.duration_minutes).await?;

        let session_id = Uuid::new_v4();
        let scheduled_end = request.scheduled_start + Duration::minutes(request.duration_minutes as i64);
        let now = Utc::now();

        // Create session in database
        let query = r#"
            INSERT INTO mentorship_sessions (
                session_id, mentor_id, mentee_id, title, description,
                scheduled_start, scheduled_end, status, session_type, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#;

        sqlx::query(query)
            .bind(session_id)
            .bind(request.mentor_id)
            .bind(mentee_id)
            .bind(&request.title)
            .bind(&request.description)
            .bind(request.scheduled_start)
            .bind(scheduled_end)
            .bind(&SessionStatus::Scheduled)
            .bind(&request.session_type)
            .bind(now)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create session: {}", e)))?;

        // Add participants
        self.add_session_participant(session_id, request.mentor_id, ParticipantRole::Mentor).await?;
        self.add_session_participant(session_id, mentee_id, ParticipantRole::Mentee).await?;

        // Handle recurring sessions
        if let Some(pattern) = &request.recurring_pattern {
            self.create_recurring_series(session_id, pattern).await?;
        }

        // Send notifications
        self.send_session_notifications(session_id, crate::models::NotificationType::SessionConfirmation).await?;

        // Create calendar events
        self.create_calendar_events(session_id).await?;

        // Cache session info
        self.cache_session_info(session_id).await?;

        self.get_session(session_id).await
    }

    pub async fn update_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        request: UpdateSessionRequest,
    ) -> Result<SessionResponse, AppError> {
        // Verify user has permission to update session
        self.verify_session_permission(session_id, user_id).await?;

        let mut update_fields = Vec::new();
        let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = Vec::new();
        let mut param_count = 1;

        if let Some(title) = &request.title {
            update_fields.push(format!("title = ${}", param_count));
            params.push(Box::new(title.clone()));
            param_count += 1;
        }

        if let Some(description) = &request.description {
            update_fields.push(format!("description = ${}", param_count));
            params.push(Box::new(description.clone()));
            param_count += 1;
        }

        if let Some(scheduled_start) = request.scheduled_start {
            // Check for conflicts if rescheduling
            if let Some(duration) = request.duration_minutes {
                let temp_request = SessionRequest {
                    mentor_id: Uuid::new_v4(), // Will be filled from existing session
                    title: "temp".to_string(),
                    description: None,
                    scheduled_start,
                    duration_minutes: duration,
                    session_type: SessionType::OneOnOne,
                    recurring_pattern: None,
                    max_participants: None,
                    materials: Vec::new(),
                };
                self.check_scheduling_conflicts(&temp_request).await?;
            }

            update_fields.push(format!("scheduled_start = ${}", param_count));
            params.push(Box::new(scheduled_start));
            param_count += 1;

            if let Some(duration) = request.duration_minutes {
                let scheduled_end = scheduled_start + Duration::minutes(duration as i64);
                update_fields.push(format!("scheduled_end = ${}", param_count));
                params.push(Box::new(scheduled_end));
                param_count += 1;
            }
        }

        if let Some(status) = &request.status {
            update_fields.push(format!("status = ${}", param_count));
            params.push(Box::new(status.clone()));
            param_count += 1;
        }

        if let Some(notes) = &request.notes {
            update_fields.push(format!("notes = ${}", param_count));
            params.push(Box::new(notes.clone()));
            param_count += 1;
        }

        if update_fields.is_empty() {
            return self.get_session(session_id).await;
        }

        update_fields.push(format!("updated_at = ${}", param_count));
        params.push(Box::new(Utc::now()));

        let query = format!(
            "UPDATE mentorship_sessions SET {} WHERE session_id = ${}",
            update_fields.join(", "),
            param_count + 1
        );

        // This is a simplified version - in a real implementation, you'd need to handle dynamic queries properly
        sqlx::query(&query)
            .bind(session_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update session: {}", e)))?;

        // Update cache
        self.cache_session_info(session_id).await?;

        // Send notifications if status changed
        if let Some(status) = &request.status {
            match status {
                SessionStatus::Cancelled => {
                    self.send_session_notifications(session_id, crate::models::NotificationType::SessionCancellation).await?;
                }
                SessionStatus::Rescheduled => {
                    self.send_session_notifications(session_id, crate::models::NotificationType::SessionRescheduled).await?;
                }
                _ => {}
            }
        }

        self.get_session(session_id).await
    }

    pub async fn cancel_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        reason: Option<String>,
    ) -> Result<(), AppError> {
        // Verify user has permission to cancel session
        self.verify_session_permission(session_id, user_id).await?;

        // Check cancellation window
        let session = self.get_session(session_id).await?;
        let now = Utc::now();
        let time_until_session = session.scheduled_start - now;
        
        if time_until_session < Duration::hours(24) {
            return Err(AppError::BadRequest("Cannot cancel session within 24 hours".to_string()));
        }

        // Update session status
        let query = "UPDATE mentorship_sessions SET status = $1, notes = $2, updated_at = $3 WHERE session_id = $4";
        sqlx::query(query)
            .bind(&SessionStatus::Cancelled)
            .bind(reason.as_deref().unwrap_or("Cancelled by user"))
            .bind(now)
            .bind(session_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to cancel session: {}", e)))?;

        // Send cancellation notifications
        self.send_session_notifications(session_id, crate::models::NotificationType::SessionCancellation).await?;

        // Remove from calendar
        self.remove_calendar_events(session_id).await?;

        // Update cache
        self.cache_session_info(session_id).await?;

        Ok(())
    }

    pub async fn get_session(&self, session_id: Uuid) -> Result<SessionResponse, AppError> {
        // Try cache first
        if let Ok(cached) = self.get_cached_session(session_id).await {
            return Ok(cached);
        }

        // Query database
        let query = r#"
            SELECT 
                s.session_id, s.mentor_id, s.mentee_id, s.title, s.description,
                s.scheduled_start, s.scheduled_end, s.actual_start, s.actual_end,
                s.status, s.session_type, s.whiteboard_data, s.notes,
                s.recurring_series_id, s.created_at, s.updated_at
            FROM mentorship_sessions s
            WHERE s.session_id = $1
        "#;

        let row = sqlx::query_as::<_, SessionDb>(query)
            .bind(session_id)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch session: {}", e)))?;

        let session_db = row.ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

        // Get participants
        let participants = self.get_session_participants(session_id).await?;

        // Get materials
        let materials = self.get_session_materials(session_id).await?;

        let session = SessionResponse {
            session_id: session_db.session_id,
            mentor_id: session_db.mentor_id,
            mentee_id: session_db.mentee_id,
            title: session_db.title,
            description: session_db.description,
            scheduled_start: session_db.scheduled_start,
            scheduled_end: session_db.scheduled_end,
            actual_start: session_db.actual_start,
            actual_end: session_db.actual_end,
            status: session_db.status.parse().unwrap_or(SessionStatus::Scheduled),
            session_type: session_db.session_type.parse().unwrap_or(SessionType::OneOnOne),
            participants,
            materials,
            whiteboard_id: None, // Would be extracted from whiteboard_data
            notes: session_db.notes,
            recurring_series_id: session_db.recurring_series_id,
            created_at: session_db.created_at,
            updated_at: session_db.updated_at,
        };

        // Cache the result
        self.cache_session_response(&session).await?;

        Ok(session)
    }

    // Availability Management
    pub async fn set_availability(
        &self,
        user_id: Uuid,
        availability: Vec<AvailabilityRequest>,
    ) -> Result<Vec<AvailabilityResponse>>, AppError> {
        let mut responses = Vec::new();

        for avail in availability {
            let availability_id = Uuid::new_v4();
            let now = Utc::now();

            let query = r#"
                INSERT INTO user_availability (
                    availability_id, user_id, day_of_week, start_time, end_time,
                    timezone, is_available, created_at, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (user_id, day_of_week, start_time) 
                DO UPDATE SET 
                    end_time = EXCLUDED.end_time,
                    is_available = EXCLUDED.is_available,
                    updated_at = EXCLUDED.updated_at
                RETURNING availability_id, created_at, updated_at
            "#;

            let row = sqlx::query_as::<_, (Uuid, DateTime<Utc>, DateTime<Utc>)>(query)
                .bind(availability_id)
                .bind(user_id)
                .bind(avail.day_of_week as i16)
                .bind(avail.start_time)
                .bind(avail.end_time)
                .bind(&avail.timezone)
                .bind(avail.is_available)
                .bind(now)
                .bind(now)
                .fetch_one(&self.db_pool)
                .await
                .map_err(|e| AppError::Database(format!("Failed to set availability: {}", e)))?;

            responses.push(AvailabilityResponse {
                availability_id: row.0,
                user_id,
                day_of_week: avail.day_of_week,
                start_time: avail.start_time,
                end_time: avail.end_time,
                timezone: avail.timezone,
                is_available: avail.is_available,
                created_at: row.1,
                updated_at: row.2,
            });
        }

        // Clear availability cache
        let cache_key = format!("availability:{}", user_id);
        let _: () = self.redis_service.del(&cache_key).await
            .map_err(|e| AppError::Internal(format!("Failed to clear availability cache: {}", e)))?;

        Ok(responses)
    }

    pub async fn get_available_slots(
        &self,
        mentor_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        duration_minutes: u32,
    ) -> Result<Vec<AvailabilitySlot>, AppError> {
        // Get mentor's availability
        let availability = self.get_user_availability(mentor_id).await?;
        
        // Get existing sessions in the date range
        let existing_sessions = self.get_sessions_in_range(mentor_id, start_date, end_date).await?;

        let mut available_slots = Vec::new();
        let mut current_date = start_date.date_naive();
        let end_date_naive = end_date.date_naive();

        while current_date <= end_date_naive {
            let weekday = current_date.weekday().num_days_from_sunday() as u8;
            
            // Find availability for this day of week
            for avail in &availability {
                if avail.day_of_week == weekday && avail.is_available {
                    // Generate time slots for this availability window
                    let slots = self.generate_time_slots(
                        current_date,
                        avail.start_time,
                        avail.end_time,
                        duration_minutes,
                        &existing_sessions,
                    );
                    available_slots.extend(slots);
                }
            }

            current_date = current_date.succ_opt().unwrap_or(current_date);
        }

        Ok(available_slots)
    }

    // Recurring Sessions
    pub async fn create_recurring_series(
        &self,
        initial_session_id: Uuid,
        pattern: &RecurringPattern,
    ) -> Result<Uuid, AppError> {
        let series_id = Uuid::new_v4();
        let now = Utc::now();

        // Create recurring series record
        let query = r#"
            INSERT INTO recurring_series (
                series_id, initial_session_id, frequency, interval_value,
                days_of_week, end_date, max_occurrences, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#;

        let days_of_week_json = pattern.days_of_week.as_ref()
            .map(|days| serde_json::to_value(days).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null);

        sqlx::query(query)
            .bind(series_id)
            .bind(initial_session_id)
            .bind(&pattern.frequency)
            .bind(pattern.interval as i32)
            .bind(days_of_week_json)
            .bind(pattern.end_date)
            .bind(pattern.max_occurrences.map(|x| x as i32))
            .bind(now)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create recurring series: {}", e)))?;

        // Update initial session with series ID
        let update_query = "UPDATE mentorship_sessions SET recurring_series_id = $1 WHERE session_id = $2";
        sqlx::query(update_query)
            .bind(series_id)
            .bind(initial_session_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update session with series ID: {}", e)))?;

        // Schedule future sessions
        self.schedule_recurring_sessions(series_id, initial_session_id, pattern).await?;

        Ok(series_id)
    }

    // Helper methods
    async fn validate_session_request(&self, request: &SessionRequest) -> Result<(), AppError> {
        // Check if scheduled time is in the future
        if request.scheduled_start <= Utc::now() {
            return Err(AppError::BadRequest("Session must be scheduled in the future".to_string()));
        }

        // Check duration limits
        if request.duration_minutes < 15 || request.duration_minutes > 240 {
            return Err(AppError::BadRequest("Session duration must be between 15 minutes and 4 hours".to_string()));
        }

        // Check advance booking limit
        let max_advance = Utc::now() + Duration::days(30);
        if request.scheduled_start > max_advance {
            return Err(AppError::BadRequest("Cannot book sessions more than 30 days in advance".to_string()));
        }

        Ok(())
    }

    async fn check_scheduling_conflicts(&self, request: &SessionRequest) -> Result<(), AppError> {
        let session_end = request.scheduled_start + Duration::minutes(request.duration_minutes as i64);

        let query = r#"
            SELECT COUNT(*) as conflict_count
            FROM mentorship_sessions
            WHERE (mentor_id = $1 OR mentee_id = $1)
            AND status NOT IN ('cancelled', 'completed')
            AND (
                (scheduled_start <= $2 AND scheduled_end > $2) OR
                (scheduled_start < $3 AND scheduled_end >= $3) OR
                (scheduled_start >= $2 AND scheduled_end <= $3)
            )
        "#;

        let row = sqlx::query_as::<_, (i64,)>(query)
            .bind(request.mentor_id)
            .bind(request.scheduled_start)
            .bind(session_end)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to check conflicts: {}", e)))?;

        if row.0 > 0 {
            return Err(AppError::BadRequest("Scheduling conflict detected".to_string()));
        }

        Ok(())
    }

    async fn check_mentor_availability(
        &self,
        mentor_id: Uuid,
        scheduled_start: &DateTime<Utc>,
        duration_minutes: u32,
    ) -> Result<(), AppError> {
        let weekday = scheduled_start.weekday().num_days_from_sunday() as u8;
        let start_time = scheduled_start.time();
        let end_time = (scheduled_start + Duration::minutes(duration_minutes as i64)).time();

        let query = r#"
            SELECT COUNT(*) as available_count
            FROM user_availability
            WHERE user_id = $1
            AND day_of_week = $2
            AND is_available = true
            AND start_time <= $3
            AND end_time >= $4
        "#;

        let row = sqlx::query_as::<_, (i64,)>(query)
            .bind(mentor_id)
            .bind(weekday as i16)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to check availability: {}", e)))?;

        if row.0 == 0 {
            return Err(AppError::BadRequest("Mentor is not available at the requested time".to_string()));
        }

        Ok(())
    }

    // Additional helper methods would be implemented here...
    async fn add_session_participant(&self, session_id: Uuid, user_id: Uuid, role: ParticipantRole) -> Result<(), AppError> {
        // Implementation for adding participants
        Ok(())
    }

    async fn send_session_notifications(&self, session_id: Uuid, notification_type: crate::models::NotificationType) -> Result<(), AppError> {
        // Implementation for sending notifications
        Ok(())
    }

    async fn create_calendar_events(&self, session_id: Uuid) -> Result<(), AppError> {
        // Implementation for creating calendar events
        Ok(())
    }

    async fn remove_calendar_events(&self, session_id: Uuid) -> Result<(), AppError> {
        // Implementation for removing calendar events
        Ok(())
    }

    async fn cache_session_info(&self, session_id: Uuid) -> Result<(), AppError> {
        // Implementation for caching session info
        Ok(())
    }

    async fn get_cached_session(&self, session_id: Uuid) -> Result<SessionResponse, AppError> {
        // Implementation for getting cached session
        Err(AppError::NotFound("Not in cache".to_string()))
    }

    async fn cache_session_response(&self, session: &SessionResponse) -> Result<(), AppError> {
        // Implementation for caching session response
        Ok(())
    }

    async fn verify_session_permission(&self, session_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        // Implementation for verifying permissions
        Ok(())
    }

    async fn get_session_participants(&self, session_id: Uuid) -> Result<Vec<SessionParticipant>, AppError> {
        // Implementation for getting participants
        Ok(Vec::new())
    }

    async fn get_session_materials(&self, session_id: Uuid) -> Result<Vec<crate::models::SessionMaterial>, AppError> {
        // Implementation for getting materials
        Ok(Vec::new())
    }

    async fn get_user_availability(&self, user_id: Uuid) -> Result<Vec<AvailabilityResponse>, AppError> {
        // Implementation for getting user availability
        Ok(Vec::new())
    }

    async fn get_sessions_in_range(&self, user_id: Uuid, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<SessionResponse>, AppError> {
        // Implementation for getting sessions in range
        Ok(Vec::new())
    }

    async fn generate_time_slots(
        &self,
        date: chrono::NaiveDate,
        start_time: NaiveTime,
        end_time: NaiveTime,
        duration_minutes: u32,
        existing_sessions: &[SessionResponse],
    ) -> Vec<AvailabilitySlot> {
        // Implementation for generating time slots
        Vec::new()
    }

    async fn schedule_recurring_sessions(&self, series_id: Uuid, initial_session_id: Uuid, pattern: &RecurringPattern) -> Result<(), AppError> {
        // Implementation for scheduling recurring sessions
        Ok(())
    }

    async fn schedule_reminder_jobs(&self, scheduler: &JobScheduler) -> Result<(), AppError> {
        // Implementation for scheduling reminder jobs
        Ok(())
    }

    async fn schedule_recurring_session_jobs(&self, scheduler: &JobScheduler) -> Result<(), AppError> {
        // Implementation for scheduling recurring session jobs
        Ok(())
    }
}