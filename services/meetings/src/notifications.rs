use uuid::Uuid;
use chrono::{DateTime, Utc};
use lettre::{
    AsyncTransport, AsyncSmtpTransport, Tokio1Executor,
    transport::smtp::authentication::Credentials,
    Message, message::{header::ContentType, Mailbox},
};

use linkwithmentor_common::AppError;
use crate::{
    config::NotificationConfig,
    models::{NotificationRequest, NotificationType, SessionResponse},
};

#[derive(Clone)]
pub struct NotificationService {
    config: NotificationConfig,
    smtp_transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
}

impl NotificationService {
    pub async fn new(config: &NotificationConfig) -> Result<Self, AppError> {
        let smtp_transport = if config.enable_email_notifications {
            let creds = Credentials::new(
                config.smtp_username.clone(),
                config.smtp_password.clone(),
            );

            let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
                .map_err(|e| AppError::Internal(format!("Failed to create SMTP transport: {}", e)))?
                .port(config.smtp_port)
                .credentials(creds)
                .build();

            Some(transport)
        } else {
            None
        };

        Ok(Self {
            config: config.clone(),
            smtp_transport,
        })
    }

    pub async fn send_notification(&self, request: NotificationRequest) -> Result<(), AppError> {
        match request.notification_type {
            NotificationType::SessionReminder => {
                self.send_session_reminder(request).await?;
            }
            NotificationType::SessionConfirmation => {
                self.send_session_confirmation(request).await?;
            }
            NotificationType::SessionCancellation => {
                self.send_session_cancellation(request).await?;
            }
            NotificationType::SessionRescheduled => {
                self.send_session_rescheduled(request).await?;
            }
            NotificationType::SessionStarted => {
                self.send_session_started(request).await?;
            }
            NotificationType::SessionEnded => {
                self.send_session_ended(request).await?;
            }
            NotificationType::MaterialShared => {
                self.send_material_shared(request).await?;
            }
            NotificationType::InvitationReceived => {
                self.send_invitation_received(request).await?;
            }
        }

        Ok(())
    }

    async fn send_session_reminder(&self, request: NotificationRequest) -> Result<(), AppError> {
        if !self.config.enable_email_notifications {
            return Ok(());
        }

        let subject = format!("Reminder: {} starting soon", request.title);
        let body = self.create_reminder_email_body(&request);

        self.send_email(request.recipient_id, &subject, &body).await?;

        tracing::info!("Sent session reminder to user {}", request.recipient_id);
        Ok(())
    }

    async fn send_session_confirmation(&self, request: NotificationRequest) -> Result<(), AppError> {
        if !self.config.enable_email_notifications {
            return Ok(());
        }

        let subject = format!("Session Confirmed: {}", request.title);
        let body = self.create_confirmation_email_body(&request);

        self.send_email(request.recipient_id, &subject, &body).await?;

        tracing::info!("Sent session confirmation to user {}", request.recipient_id);
        Ok(())
    }

    async fn send_session_cancellation(&self, request: NotificationRequest) -> Result<(), AppError> {
        if !self.config.enable_email_notifications {
            return Ok(());
        }

        let subject = format!("Session Cancelled: {}", request.title);
        let body = self.create_cancellation_email_body(&request);

        self.send_email(request.recipient_id, &subject, &body).await?;

        tracing::info!("Sent session cancellation to user {}", request.recipient_id);
        Ok(())
    }

    async fn send_session_rescheduled(&self, request: NotificationRequest) -> Result<(), AppError> {
        if !self.config.enable_email_notifications {
            return Ok(());
        }

        let subject = format!("Session Rescheduled: {}", request.title);
        let body = self.create_rescheduled_email_body(&request);

        self.send_email(request.recipient_id, &subject, &body).await?;

        tracing::info!("Sent session rescheduled notification to user {}", request.recipient_id);
        Ok(())
    }

    async fn send_session_started(&self, request: NotificationRequest) -> Result<(), AppError> {
        if !self.config.enable_email_notifications {
            return Ok(());
        }

        let subject = format!("Session Started: {}", request.title);
        let body = self.create_session_started_email_body(&request);

        self.send_email(request.recipient_id, &subject, &body).await?;

        tracing::info!("Sent session started notification to user {}", request.recipient_id);
        Ok(())
    }

    async fn send_session_ended(&self, request: NotificationRequest) -> Result<(), AppError> {
        if !self.config.enable_email_notifications {
            return Ok(());
        }

        let subject = format!("Session Completed: {}", request.title);
        let body = self.create_session_ended_email_body(&request);

        self.send_email(request.recipient_id, &subject, &body).await?;

        tracing::info!("Sent session ended notification to user {}", request.recipient_id);
        Ok(())
    }

    async fn send_material_shared(&self, request: NotificationRequest) -> Result<(), AppError> {
        if !self.config.enable_email_notifications {
            return Ok(());
        }

        let subject = format!("New Material Shared: {}", request.title);
        let body = self.create_material_shared_email_body(&request);

        self.send_email(request.recipient_id, &subject, &body).await?;

        tracing::info!("Sent material shared notification to user {}", request.recipient_id);
        Ok(())
    }

    async fn send_invitation_received(&self, request: NotificationRequest) -> Result<(), AppError> {
        if !self.config.enable_email_notifications {
            return Ok(());
        }

        let subject = format!("Session Invitation: {}", request.title);
        let body = self.create_invitation_email_body(&request);

        self.send_email(request.recipient_id, &subject, &body).await?;

        tracing::info!("Sent session invitation to user {}", request.recipient_id);
        Ok(())
    }

    async fn send_email(&self, recipient_id: Uuid, subject: &str, body: &str) -> Result<(), AppError> {
        let transport = self.smtp_transport.as_ref()
            .ok_or_else(|| AppError::Internal("SMTP transport not configured".to_string()))?;

        // In a real implementation, you would fetch the user's email from the database
        let recipient_email = format!("user_{}@example.com", recipient_id);

        let from_mailbox: Mailbox = format!("{} <{}>", self.config.from_name, self.config.from_email)
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid from email: {}", e)))?;

        let to_mailbox: Mailbox = recipient_email
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid recipient email: {}", e)))?;

        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body.to_string())
            .map_err(|e| AppError::Internal(format!("Failed to build email: {}", e)))?;

        transport.send(email).await
            .map_err(|e| AppError::Internal(format!("Failed to send email: {}", e)))?;

        Ok(())
    }

    // Email template methods
    fn create_reminder_email_body(&self, request: &NotificationRequest) -> String {
        format!(
            r#"
            <html>
            <body>
                <h2>Session Reminder</h2>
                <p>Hello,</p>
                <p>This is a reminder that your session "<strong>{}</strong>" is starting soon.</p>
                <p>{}</p>
                <p>Please make sure you're ready to join at the scheduled time.</p>
                <p>Best regards,<br>The LinkWithMentor Team</p>
            </body>
            </html>
            "#,
            request.title,
            request.message
        )
    }

    fn create_confirmation_email_body(&self, request: &NotificationRequest) -> String {
        format!(
            r#"
            <html>
            <body>
                <h2>Session Confirmed</h2>
                <p>Hello,</p>
                <p>Your session "<strong>{}</strong>" has been confirmed.</p>
                <p>{}</p>
                <p>We look forward to your session!</p>
                <p>Best regards,<br>The LinkWithMentor Team</p>
            </body>
            </html>
            "#,
            request.title,
            request.message
        )
    }

    fn create_cancellation_email_body(&self, request: &NotificationRequest) -> String {
        format!(
            r#"
            <html>
            <body>
                <h2>Session Cancelled</h2>
                <p>Hello,</p>
                <p>Unfortunately, your session "<strong>{}</strong>" has been cancelled.</p>
                <p>{}</p>
                <p>Please feel free to reschedule at your convenience.</p>
                <p>Best regards,<br>The LinkWithMentor Team</p>
            </body>
            </html>
            "#,
            request.title,
            request.message
        )
    }

    fn create_rescheduled_email_body(&self, request: &NotificationRequest) -> String {
        format!(
            r#"
            <html>
            <body>
                <h2>Session Rescheduled</h2>
                <p>Hello,</p>
                <p>Your session "<strong>{}</strong>" has been rescheduled.</p>
                <p>{}</p>
                <p>Please note the new time and make sure you're available.</p>
                <p>Best regards,<br>The LinkWithMentor Team</p>
            </body>
            </html>
            "#,
            request.title,
            request.message
        )
    }

    fn create_session_started_email_body(&self, request: &NotificationRequest) -> String {
        format!(
            r#"
            <html>
            <body>
                <h2>Session Started</h2>
                <p>Hello,</p>
                <p>Your session "<strong>{}</strong>" has started.</p>
                <p>{}</p>
                <p>Join now to participate!</p>
                <p>Best regards,<br>The LinkWithMentor Team</p>
            </body>
            </html>
            "#,
            request.title,
            request.message
        )
    }

    fn create_session_ended_email_body(&self, request: &NotificationRequest) -> String {
        format!(
            r#"
            <html>
            <body>
                <h2>Session Completed</h2>
                <p>Hello,</p>
                <p>Your session "<strong>{}</strong>" has been completed.</p>
                <p>{}</p>
                <p>Thank you for using LinkWithMentor!</p>
                <p>Best regards,<br>The LinkWithMentor Team</p>
            </body>
            </html>
            "#,
            request.title,
            request.message
        )
    }

    fn create_material_shared_email_body(&self, request: &NotificationRequest) -> String {
        format!(
            r#"
            <html>
            <body>
                <h2>New Material Shared</h2>
                <p>Hello,</p>
                <p>New material has been shared for your session "<strong>{}</strong>".</p>
                <p>{}</p>
                <p>You can access it in your session dashboard.</p>
                <p>Best regards,<br>The LinkWithMentor Team</p>
            </body>
            </html>
            "#,
            request.title,
            request.message
        )
    }

    fn create_invitation_email_body(&self, request: &NotificationRequest) -> String {
        format!(
            r#"
            <html>
            <body>
                <h2>Session Invitation</h2>
                <p>Hello,</p>
                <p>You've been invited to join the session "<strong>{}</strong>".</p>
                <p>{}</p>
                <p>Please confirm your attendance.</p>
                <p>Best regards,<br>The LinkWithMentor Team</p>
            </body>
            </html>
            "#,
            request.title,
            request.message
        )
    }

    // SMS notification methods (placeholder implementations)
    pub async fn send_sms_notification(&self, phone_number: &str, message: &str) -> Result<(), AppError> {
        if !self.config.enable_sms_notifications {
            return Ok(());
        }

        // In a real implementation, you would integrate with SMS providers like Twilio
        tracing::info!("SMS notification sent to {}: {}", phone_number, message);
        Ok(())
    }

    // Bulk notification methods
    pub async fn send_bulk_notifications(&self, requests: Vec<NotificationRequest>) -> Result<(), AppError> {
        for request in requests {
            if let Err(e) = self.send_notification(request).await {
                tracing::error!("Failed to send notification: {}", e);
                // Continue with other notifications even if one fails
            }
        }
        Ok(())
    }

    // Scheduled notification methods
    pub async fn schedule_notification(&self, request: NotificationRequest, send_at: DateTime<Utc>) -> Result<(), AppError> {
        // In a real implementation, you would store this in a job queue or scheduler
        tracing::info!("Scheduled notification for {} at {}", request.recipient_id, send_at);
        Ok(())
    }

    // Template management
    pub fn get_notification_template(&self, notification_type: &NotificationType) -> String {
        match notification_type {
            NotificationType::SessionReminder => "session_reminder".to_string(),
            NotificationType::SessionConfirmation => "session_confirmation".to_string(),
            NotificationType::SessionCancellation => "session_cancellation".to_string(),
            NotificationType::SessionRescheduled => "session_rescheduled".to_string(),
            NotificationType::SessionStarted => "session_started".to_string(),
            NotificationType::SessionEnded => "session_ended".to_string(),
            NotificationType::MaterialShared => "material_shared".to_string(),
            NotificationType::InvitationReceived => "invitation_received".to_string(),
        }
    }

    // Notification preferences
    pub async fn get_user_notification_preferences(&self, user_id: Uuid) -> Result<HashMap<String, bool>, AppError> {
        // In a real implementation, you would fetch from database
        let mut preferences = HashMap::new();
        preferences.insert("email_reminders".to_string(), true);
        preferences.insert("sms_reminders".to_string(), false);
        preferences.insert("email_confirmations".to_string(), true);
        preferences.insert("push_notifications".to_string(), true);
        
        Ok(preferences)
    }

    pub async fn update_user_notification_preferences(
        &self,
        user_id: Uuid,
        preferences: HashMap<String, bool>,
    ) -> Result<(), AppError> {
        // In a real implementation, you would update the database
        tracing::info!("Updated notification preferences for user {}: {:?}", user_id, preferences);
        Ok(())
    }
}