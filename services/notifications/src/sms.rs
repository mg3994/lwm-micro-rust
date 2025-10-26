use reqwest::Client;
use std::collections::HashMap;

use linkwithmentor_common::AppError;
use crate::config::SmsConfig;

#[derive(Clone)]
pub struct SmsService {
    client: Client,
    config: SmsConfig,
}

impl SmsService {
    pub async fn new(config: &SmsConfig) -> Result<Self, AppError> {
        Ok(Self {
            client: Client::new(),
            config: config.clone(),
        })
    }

    pub async fn send_sms(&self, to: &str, message: &str) -> Result<(), AppError> {
        if !self.config.enabled {
            tracing::info!("SMS service disabled, skipping SMS to: {}", to);
            return Ok(());
        }

        // Placeholder implementation - would integrate with actual SMS provider
        tracing::info!("SMS sent to {}: {}", to, message);
        Ok(())
    }
}