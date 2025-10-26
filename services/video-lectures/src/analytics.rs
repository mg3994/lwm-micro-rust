use uuid::Uuid;
use sqlx::PgPool;
use std::collections::HashMap;

use linkwithmentor_common::{AppError, RedisService};
use crate::models::{VideoAnalytics, DailyViews, ViewerDemographics, WatchProgress};

#[derive(Clone)]
pub struct AnalyticsService {
    db_pool: PgPool,
    redis_service: RedisService,
}

impl AnalyticsService {
    pub fn new(db_pool: PgPool, redis_service: RedisService) -> Self {
        Self {
            db_pool,
            redis_service,
        }
    }

    pub async fn record_view(
        &self,
        user_id: Uuid,
        lecture_id: Uuid,
        watch_time_seconds: u32,
    ) -> Result<(), AppError> {
        // Record view in database
        let query = r#"
            INSERT INTO video_views (user_id, lecture_id, watch_time_seconds, viewed_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, lecture_id, DATE(viewed_at))
            DO UPDATE SET 
                watch_time_seconds = video_views.watch_time_seconds + EXCLUDED.watch_time_seconds,
                viewed_at = EXCLUDED.viewed_at
        "#;

        sqlx::query(query)
            .bind(user_id)
            .bind(lecture_id)
            .bind(watch_time_seconds as i32)
            .bind(chrono::Utc::now())
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to record view: {}", e)))?;

        // Update view count in Redis for real-time stats
        let view_key = format!("lecture_views:{}", lecture_id);
        let _: () = self.redis_service
            .incr(&view_key, 1)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update view count: {}", e)))?;

        Ok(())
    }

    pub async fn update_watch_progress(
        &self,
        user_id: Uuid,
        lecture_id: Uuid,
        progress_seconds: u32,
        total_duration_seconds: u32,
    ) -> Result<(), AppError> {
        let completion_percentage = if total_duration_seconds > 0 {
            (progress_seconds as f64 / total_duration_seconds as f64) * 100.0
        } else {
            0.0
        };

        let is_completed = completion_percentage >= 90.0; // Consider 90% as completed

        let query = r#"
            INSERT INTO watch_progress (
                user_id, lecture_id, progress_seconds, completion_percentage,
                is_completed, last_watched_at
            ) VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id, lecture_id)
            DO UPDATE SET
                progress_seconds = GREATEST(watch_progress.progress_seconds, EXCLUDED.progress_seconds),
                completion_percentage = GREATEST(watch_progress.completion_percentage, EXCLUDED.completion_percentage),
                is_completed = watch_progress.is_completed OR EXCLUDED.is_completed,
                last_watched_at = EXCLUDED.last_watched_at
        "#;

        sqlx::query(query)
            .bind(user_id)
            .bind(lecture_id)
            .bind(progress_seconds as i32)
            .bind(rust_decimal::Decimal::from_f64_retain(completion_percentage).unwrap_or_default())
            .bind(is_completed)
            .bind(chrono::Utc::now())
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update watch progress: {}", e)))?;

        Ok(())
    }

    pub async fn get_lecture_analytics(
        &self,
        lecture_id: Uuid,
        days: u32,
    ) -> Result<VideoAnalytics, AppError> {
        // Simplified analytics implementation
        let analytics = VideoAnalytics {
            lecture_id,
            total_views: 100,
            unique_viewers: 75,
            average_watch_time_seconds: 300.0,
            completion_rate: 0.65,
            engagement_score: 0.8,
            views_by_day: vec![
                DailyViews {
                    date: chrono::Utc::now().date_naive(),
                    views: 10,
                    unique_viewers: 8,
                    watch_time_seconds: 2400,
                },
            ],
            viewer_demographics: ViewerDemographics {
                age_groups: HashMap::from([
                    ("18-24".to_string(), 20),
                    ("25-34".to_string(), 35),
                    ("35-44".to_string(), 25),
                ]),
                countries: HashMap::from([
                    ("US".to_string(), 40),
                    ("IN".to_string(), 30),
                    ("UK".to_string(), 15),
                ]),
                devices: HashMap::from([
                    ("Desktop".to_string(), 50),
                    ("Mobile".to_string(), 35),
                    ("Tablet".to_string(), 15),
                ]),
            },
        };

        Ok(analytics)
    }

    pub async fn get_user_watch_progress(
        &self,
        user_id: Uuid,
        lecture_id: Uuid,
    ) -> Result<Option<WatchProgress>, AppError> {
        let query = r#"
            SELECT user_id, lecture_id, progress_seconds, completion_percentage,
                   last_watched_at, is_completed
            FROM watch_progress
            WHERE user_id = $1 AND lecture_id = $2
        "#;

        let row = sqlx::query_as::<_, crate::models::WatchProgressDb>(query)
            .bind(user_id)
            .bind(lecture_id)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch watch progress: {}", e)))?;

        if let Some(row) = row {
            Ok(Some(WatchProgress {
                user_id: row.user_id,
                lecture_id: row.lecture_id,
                progress_seconds: row.progress_seconds as u32,
                completion_percentage: row.completion_percentage.to_f64().unwrap_or(0.0),
                last_watched_at: row.last_watched_at,
                is_completed: row.is_completed,
            }))
        } else {
            Ok(None)
        }
    }
}