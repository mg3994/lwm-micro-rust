use uuid::Uuid;
use sqlx::PgPool;
use chrono::{DateTime, Utc, Duration};

use linkwithmentor_common::AppError;
use crate::{
    models::{VideoUploadRequest, VideoUploadResponse, VideoLecture, VideoStatus},
    storage::StorageService,
    config::UploadConfig,
};

#[derive(Clone)]
pub struct UploadService {
    db_pool: PgPool,
    storage_service: StorageService,
    config: UploadConfig,
}

impl UploadService {
    pub fn new(
        db_pool: PgPool,
        storage_service: StorageService,
        config: &UploadConfig,
    ) -> Self {
        Self {
            db_pool,
            storage_service,
            config: config.clone(),
        }
    }

    pub async fn initiate_upload(
        &self,
        user_id: Uuid,
        request: VideoUploadRequest,
    ) -> Result<VideoUploadResponse, AppError> {
        // Validate file size
        if request.file_size > (self.config.max_file_size_mb * 1024 * 1024) {
            return Err(AppError::BadRequest("File size exceeds limit".to_string()));
        }

        // Validate file format
        let extension = request.filename.split('.').last().unwrap_or("");
        if !self.config.allowed_formats.contains(&extension.to_lowercase()) {
            return Err(AppError::BadRequest("Unsupported file format".to_string()));
        }

        // Generate upload ID and URL
        let upload_id = Uuid::new_v4();
        let expires_at = Utc::now() + Duration::seconds(self.config.upload_timeout_seconds as i64);

        // Create presigned upload URL
        let upload_url = self.storage_service
            .create_presigned_upload_url(&format!("uploads/{}", upload_id), expires_at)
            .await?;

        // Update lecture status to uploading
        let query = "UPDATE video_lectures SET status = $1, updated_at = $2 WHERE lecture_id = $3";
        sqlx::query(query)
            .bind(&VideoStatus::Uploading)
            .bind(Utc::now())
            .bind(request.lecture_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update lecture status: {}", e)))?;

        Ok(VideoUploadResponse {
            upload_id,
            upload_url,
            expires_at,
            chunk_size: self.config.chunk_size_mb * 1024 * 1024,
        })
    }

    pub async fn complete_upload(
        &self,
        upload_id: Uuid,
        lecture_id: Uuid,
    ) -> Result<(), AppError> {
        // Move file from temp to permanent storage
        let temp_path = format!("uploads/{}", upload_id);
        let permanent_path = format!("videos/{}/original", lecture_id);

        self.storage_service
            .move_file(&temp_path, &permanent_path)
            .await?;

        // Update lecture status and trigger processing
        let query = r#"
            UPDATE video_lectures 
            SET status = $1, updated_at = $2 
            WHERE lecture_id = $3
        "#;

        sqlx::query(query)
            .bind(&VideoStatus::Processing)
            .bind(Utc::now())
            .bind(lecture_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update lecture: {}", e)))?;

        tracing::info!("Upload completed for lecture {}", lecture_id);
        Ok(())
    }
}