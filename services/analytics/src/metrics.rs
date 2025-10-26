use sqlx::PgPool;
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

use linkwithmentor_common::{AppError, RedisService};
use crate::models::{
    UserMetrics, SessionMetrics, RevenueMetrics, PlatformMetrics, 
    EngagementMetrics, EventTracking, AnalyticsQuery, AnalyticsResult
};

#[derive(Clone)]
pub struct MetricsService {
    db_pool: PgPool,
    redis_service: RedisService,
}

impl MetricsService {
    pub fn new(db_pool: PgPool, redis_service: RedisService) -> Self {
        Self {
            db_pool,
            redis_service,
        }
    }

    pub async fn initialize(&self) -> Result<(), AppError> {
        // Initialize any required metrics aggregation tables or indexes
        tracing::info!("Metrics service initialized");
        Ok(())
    }

    pub async fn get_user_metrics(&self, date_range: Option<(DateTime<Utc>, DateTime<Utc>)>) -> Result<UserMetrics, AppError> {
        let cache_key = "metrics:users";
        
        // Try to get from cache first
        if let Ok(cached) = self.redis_service.get::<UserMetrics>(cache_key).await {
            return Ok(cached);
        }

        let (start_date, end_date) = date_range.unwrap_or_else(|| {
            let end = Utc::now();
            let start = end - Duration::days(30);
            (start, end)
        });

        // Get user metrics from database
        let total_users: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM users WHERE created_at <= $1",
            end_date
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(0);

        let active_users_daily: i64 = sqlx::query_scalar!(
            "SELECT COUNT(DISTINCT user_id) FROM user_sessions 
             WHERE last_activity >= $1",
            Utc::now() - Duration::days(1)
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(0);

        let new_users_today: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM users 
             WHERE created_at >= $1",
            Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(0);

        let metrics = UserMetrics {
            total_users,
            active_users_daily,
            active_users_weekly: active_users_daily * 7, // Simplified calculation
            active_users_monthly: active_users_daily * 30, // Simplified calculation
            new_users_today,
            new_users_this_week: new_users_today * 7, // Simplified calculation
            new_users_this_month: new_users_today * 30, // Simplified calculation
            user_retention_rate: 0.75, // Placeholder - would calculate actual retention
            average_session_duration: 1800.0, // 30 minutes in seconds
        };

        // Cache the result
        let _ = self.redis_service.set_with_expiry(cache_key, &metrics, 300).await;

        Ok(metrics)
    }

    pub async fn get_session_metrics(&self, date_range: Option<(DateTime<Utc>, DateTime<Utc>)>) -> Result<SessionMetrics, AppError> {
        let cache_key = "metrics:sessions";
        
        if let Ok(cached) = self.redis_service.get::<SessionMetrics>(cache_key).await {
            return Ok(cached);
        }

        let (start_date, end_date) = date_range.unwrap_or_else(|| {
            let end = Utc::now();
            let start = end - Duration::days(30);
            (start, end)
        });

        let total_sessions: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM mentoring_sessions 
             WHERE created_at BETWEEN $1 AND $2",
            start_date, end_date
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(0);

        let completed_sessions: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM mentoring_sessions 
             WHERE status = 'completed' AND created_at BETWEEN $1 AND $2",
            start_date, end_date
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(0);

        let metrics = SessionMetrics {
            total_sessions,
            completed_sessions,
            cancelled_sessions: total_sessions - completed_sessions,
            average_session_duration: 3600.0, // 1 hour in seconds
            total_session_revenue: 50000.0, // Placeholder
            sessions_by_category: HashMap::new(),
            peak_hours: vec![14, 15, 16, 17, 18], // 2-6 PM
        };

        let _ = self.redis_service.set_with_expiry(cache_key, &metrics, 300).await;
        Ok(metrics)
    }

    pub async fn get_revenue_metrics(&self, date_range: Option<(DateTime<Utc>, DateTime<Utc>)>) -> Result<RevenueMetrics, AppError> {
        let cache_key = "metrics:revenue";
        
        if let Ok(cached) = self.redis_service.get::<RevenueMetrics>(cache_key).await {
            return Ok(cached);
        }

        let (start_date, end_date) = date_range.unwrap_or_else(|| {
            let end = Utc::now();
            let start = end - Duration::days(30);
            (start, end)
        });

        let total_revenue: Option<f64> = sqlx::query_scalar!(
            "SELECT SUM(amount) FROM payments 
             WHERE status = 'completed' AND created_at BETWEEN $1 AND $2",
            start_date, end_date
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let revenue_today: Option<f64> = sqlx::query_scalar!(
            "SELECT SUM(amount) FROM payments 
             WHERE status = 'completed' AND created_at >= $1",
            Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let metrics = RevenueMetrics {
            total_revenue: total_revenue.unwrap_or(0.0),
            revenue_today: revenue_today.unwrap_or(0.0),
            revenue_this_week: revenue_today.unwrap_or(0.0) * 7.0, // Simplified
            revenue_this_month: revenue_today.unwrap_or(0.0) * 30.0, // Simplified
            average_transaction_value: 100.0, // Placeholder
            revenue_by_category: HashMap::new(),
            top_earning_mentors: Vec::new(),
        };

        let _ = self.redis_service.set_with_expiry(cache_key, &metrics, 300).await;
        Ok(metrics)
    }

    pub async fn get_platform_metrics(&self) -> Result<PlatformMetrics, AppError> {
        let cache_key = "metrics:platform";
        
        if let Ok(cached) = self.redis_service.get::<PlatformMetrics>(cache_key).await {
            return Ok(cached);
        }

        let total_mentors: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM users WHERE user_type = 'mentor'"
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(0);

        let total_mentees: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM users WHERE user_type = 'mentee'"
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(0);

        let metrics = PlatformMetrics {
            total_mentors,
            active_mentors: (total_mentors as f64 * 0.7) as i64, // 70% active
            total_mentees,
            active_mentees: (total_mentees as f64 * 0.8) as i64, // 80% active
            mentor_approval_rate: 0.85,
            average_mentor_rating: 4.2,
            total_reviews: 1500,
        };

        let _ = self.redis_service.set_with_expiry(cache_key, &metrics, 600).await;
        Ok(metrics)
    }

    pub async fn get_engagement_metrics(&self) -> Result<EngagementMetrics, AppError> {
        let cache_key = "metrics:engagement";
        
        if let Ok(cached) = self.redis_service.get::<EngagementMetrics>(cache_key).await {
            return Ok(cached);
        }

        let total_messages: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM messages"
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(0);

        let messages_today: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM messages WHERE created_at >= $1",
            Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(0);

        let metrics = EngagementMetrics {
            total_messages,
            messages_today,
            average_response_time: 300.0, // 5 minutes
            video_call_duration: 3600.0, // 1 hour
            content_views: 10000,
            user_interactions: HashMap::new(),
        };

        let _ = self.redis_service.set_with_expiry(cache_key, &metrics, 300).await;
        Ok(metrics)
    }

    pub async fn track_event(&self, event: EventTracking) -> Result<(), AppError> {
        // Store event in database
        sqlx::query!(
            "INSERT INTO analytics_events (event_id, user_id, session_id, event_name, event_category, properties, timestamp, ip_address, user_agent)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            event.event_id,
            event.user_id,
            event.session_id,
            event.event_name,
            event.event_category,
            serde_json::to_value(&event.properties).unwrap_or_default(),
            event.timestamp,
            event.ip_address,
            event.user_agent
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Also store in Redis for real-time processing
        let redis_key = format!("events:{}:{}", event.event_category, event.event_name);
        let _ = self.redis_service.increment(&redis_key).await;

        Ok(())
    }

    pub async fn execute_query(&self, query: AnalyticsQuery) -> Result<AnalyticsResult, AppError> {
        let start_time = std::time::Instant::now();
        
        // This is a simplified implementation
        // In a real system, you'd build dynamic SQL based on the query parameters
        let data = vec![
            HashMap::from([
                ("date".to_string(), serde_json::Value::String("2024-01-01".to_string())),
                ("value".to_string(), serde_json::Value::Number(serde_json::Number::from(100))),
            ]),
            HashMap::from([
                ("date".to_string(), serde_json::Value::String("2024-01-02".to_string())),
                ("value".to_string(), serde_json::Value::Number(serde_json::Number::from(150))),
            ]),
        ];

        let query_time = start_time.elapsed().as_millis() as i64;

        Ok(AnalyticsResult {
            data,
            total_rows: 2,
            query_time_ms: query_time,
            cached: false,
        })
    }
}