use chrono::{DateTime, Utc};
use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client, presigning::PresigningConfig};

use linkwithmentor_common::AppError;
use crate::config::StorageConfig;

#[derive(Clone)]
pub struct StorageService {
    s3_client: Option<Client>,
    config: StorageConfig,
}

impl StorageService {
    pub async fn new(config: &StorageConfig) -> Result<Self, AppError> {
        let s3_client = if config.provider == "s3" {
            let aws_config = aws_config::defaults(BehaviorVersion::latest())
                .region(aws_config::Region::new(config.region.clone()))
                .load()
                .await;
            Some(Client::new(&aws_config))
        } else {
            None
        };

        Ok(Self {
            s3_client,
            config: config.clone(),
        })
    }

    pub async fn create_presigned_upload_url(
        &self,
        key: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<String, AppError> {
        if let Some(client) = &self.s3_client {
            let duration = expires_at - Utc::now();
            let presigning_config = PresigningConfig::expires_in(
                std::time::Duration::from_secs(duration.num_seconds() as u64)
            ).map_err(|e| AppError::Internal(format!("Failed to create presigning config: {}", e)))?;

            let presigned_request = client
                .put_object()
                .bucket(&self.config.bucket_name)
                .key(key)
                .presigned(presigning_config)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to create presigned URL: {}", e)))?;

            Ok(presigned_request.uri().to_string())
        } else {
            // For local storage, return a placeholder URL
            Ok(format!("http://localhost:8006/upload/{}", key))
        }
    }

    pub async fn move_file(&self, from_key: &str, to_key: &str) -> Result<(), AppError> {
        if let Some(client) = &self.s3_client {
            // Copy object to new location
            let copy_source = format!("{}/{}", self.config.bucket_name, from_key);
            client
                .copy_object()
                .bucket(&self.config.bucket_name)
                .key(to_key)
                .copy_source(&copy_source)
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to copy object: {}", e)))?;

            // Delete original object
            client
                .delete_object()
                .bucket(&self.config.bucket_name)
                .key(from_key)
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to delete original object: {}", e)))?;
        }

        Ok(())
    }

    pub async fn get_file_url(&self, key: &str) -> String {
        if let Some(cdn_domain) = &self.config.cdn_domain {
            format!("https://{}/{}", cdn_domain, key)
        } else {
            format!("https://{}.s3.{}.amazonaws.com/{}", 
                self.config.bucket_name, self.config.region, key)
        }
    }

    pub async fn delete_file(&self, key: &str) -> Result<(), AppError> {
        if let Some(client) = &self.s3_client {
            client
                .delete_object()
                .bucket(&self.config.bucket_name)
                .key(key)
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to delete object: {}", e)))?;
        }

        Ok(())
    }
}