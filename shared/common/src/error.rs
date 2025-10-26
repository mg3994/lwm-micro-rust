use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    #[error("Authorization error: {0}")]
    Authorization(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Conflict: {0}")]
    Conflict(String),
    
    #[error("Payment error: {0}")]
    Payment(String),
    
    #[error("External service error: {0}")]
    ExternalService(String),
    
    #[error("Internal server error: {0}")]
    Internal(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub error_code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub request_id: String,
}

impl ApiError {
    pub fn new(error_code: String, message: String) -> Self {
        Self {
            error_code,
            message,
            details: None,
            timestamp: Utc::now(),
            request_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

// HTTP status code mapping
impl AppError {
    pub fn status_code(&self) -> u16 {
        match self {
            AppError::Authentication(_) => 401,
            AppError::Authorization(_) => 403,
            AppError::NotFound(_) => 404,
            AppError::Validation(_) => 400,
            AppError::Conflict(_) => 409,
            AppError::Payment(_) => 402,
            AppError::ExternalService(_) => 502,
            _ => 500,
        }
    }

    pub fn error_code(&self) -> &str {
        match self {
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::Redis(_) => "CACHE_ERROR",
            AppError::Authentication(_) => "AUTHENTICATION_ERROR",
            AppError::Authorization(_) => "AUTHORIZATION_ERROR",
            AppError::Validation(_) => "VALIDATION_ERROR",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Conflict(_) => "CONFLICT",
            AppError::Payment(_) => "PAYMENT_ERROR",
            AppError::ExternalService(_) => "EXTERNAL_SERVICE_ERROR",
            AppError::Internal(_) => "INTERNAL_ERROR",
        }
    }
}