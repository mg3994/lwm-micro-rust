use lettre::{
    message::{header::ContentType, Mailbox, Message},
    transport::smtp::{authentication::Credentials, PoolConfig},
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use std::collections::HashMap;

use linkwithmentor_common::AppError;
use crate::config::EmailConfig;

#[derive(Clone)]
pub struct EmailService {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    config: EmailConfig,
}

impl EmailService {
    pub async fn new(config: &EmailConfig) -> Result<Self, AppError> {
        if !config.enabled {
            return Ok(Self {
                transport: AsyncSmtpTransport::<Tokio1Executor>::unencrypted_localhost(),
                config: config.clone(),
            });
        }

        let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());
        
        let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            .map_err(|e| AppError::Internal(format!("SMTP relay error: {}", e)))?
            .credentials(creds)
            .pool_config(PoolConfig::new().max_size(10))
            .build();

        Ok(Self {
            transport,
            config: config.clone(),
        })
    }

    pub async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        is_html: bool,
    ) -> Result<(), AppError> {
        if !self.config.enabled {
            tracing::info!("Email service disabled, skipping email to: {}", to);
            return Ok(());
        }

        let from_mailbox: Mailbox = format!("{} <{}>", self.config.from_name, self.config.from_email)
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid from address: {}", e)))?;

        let to_mailbox: Mailbox = to.parse()
            .map_err(|e| AppError::Internal(format!("Invalid to address: {}", e)))?;

        let mut message_builder = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject);

        if let Some(reply_to) = &self.config.reply_to {
            let reply_to_mailbox: Mailbox = reply_to.parse()
                .map_err(|e| AppError::Internal(format!("Invalid reply-to address: {}", e)))?;
            message_builder = message_builder.reply_to(reply_to_mailbox);
        }

        let message = if is_html {
            message_builder
                .header(ContentType::TEXT_HTML)
                .body(body.to_string())
        } else {
            message_builder
                .header(ContentType::TEXT_PLAIN)
                .body(body.to_string())
        }
        .map_err(|e| AppError::Internal(format!("Failed to build email: {}", e)))?;

        self.transport
            .send(message)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to send email: {}", e)))?;

        tracing::info!("Email sent successfully to: {}", to);
        Ok(())
    }

    pub async fn send_templated_email(
        &self,
        to: &str,
        template_data: HashMap<String, String>,
        subject_template: &str,
        body_template: &str,
    ) -> Result<(), AppError> {
        // Simple template replacement (in production, use a proper template engine)
        let mut subject = subject_template.to_string();
        let mut body = body_template.to_string();

        for (key, value) in template_data {
            let placeholder = format!("{{{{{}}}}}", key);
            subject = subject.replace(&placeholder, &value);
            body = body.replace(&placeholder, &value);
        }

        self.send_email(to, &subject, &body, true).await
    }
}