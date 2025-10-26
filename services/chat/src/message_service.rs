use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use linkwithmentor_common::{AppError, RedisService, MessageType, ModerationStatus};
use crate::{
    models::{ChatMessageResponse, MessageHistoryResponse},
    connection_manager::ConnectionManager,
};

#[derive(Clone)]
pub struct MessageService {
    db_pool: PgPool,
    redis_service: RedisService,
    connection_manager: ConnectionManager,
}

impl MessageService {
    pub fn new(
        db_pool: PgPool,
        redis_service: RedisService,
        connection_manager: ConnectionManager,
    ) -> Self {
        Self {
            db_pool,
            redis_service,
            connection_manager,
        }
    }

    pub async fn send_message(
        &self,
        sender_id: Uuid,
        content: String,
        recipient_id: Option<Uuid>,
        session_id: Option<Uuid>,
        group_id: Option<Uuid>,
        message_type: MessageType,
    ) -> Result<ChatMessageResponse, AppError> {
        // Validate message content
        if content.trim().is_empty() {
            return Err(AppError::BadRequest("Message content cannot be empty".to_string()));
        }

        // Check rate limiting
        self.check_rate_limit(sender_id).await?;

        // Get sender information
        let sender_info = self.get_user_info(sender_id).await?;

        // Perform content moderation
        let moderation_status = self.moderate_content(&content).await?;

        // Generate message ID
        let message_id = Uuid::new_v4();
        let timestamp = Utc::now();

        // Store message in database
        let query = r#"
            INSERT INTO messages (
                message_id, sender_id, recipient_id, session_id, group_id,
                content, message_type, moderation_status, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#;

        sqlx::query(query)
            .bind(message_id)
            .bind(sender_id)
            .bind(recipient_id)
            .bind(session_id)
            .bind(group_id)
            .bind(&content)
            .bind(&message_type)
            .bind(&moderation_status)
            .bind(timestamp)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to store message: {}", e)))?;

        // Cache recent message for quick retrieval
        self.cache_recent_message(message_id, &content, sender_id, timestamp).await?;

        // Update conversation last activity
        self.update_conversation_activity(recipient_id, session_id, group_id).await?;

        // Increment rate limit counter
        self.increment_rate_limit_counter(sender_id).await?;

        Ok(ChatMessageResponse {
            message_id,
            sender_id,
            sender_username: sender_info.username,
            content,
            recipient_id,
            session_id,
            group_id,
            message_type,
            moderation_status,
            timestamp,
            edited_at: None,
            is_edited: false,
        })
    }

    pub async fn get_message_history(
        &self,
        user_id: Uuid,
        session_id: Option<Uuid>,
        group_id: Option<Uuid>,
        limit: u32,
        before_message_id: Option<Uuid>,
    ) -> Result<MessageHistoryResponse, AppError> {
        let limit = std::cmp::min(limit, 100); // Cap at 100 messages

        let mut query = String::from(r#"
            SELECT 
                m.message_id, m.sender_id, u.username as sender_username,
                m.content, m.recipient_id, m.session_id, m.group_id,
                m.message_type, m.moderation_status, m.created_at,
                m.updated_at, m.is_edited
            FROM messages m
            JOIN users u ON m.sender_id = u.user_id
            WHERE 1=1
        "#);

        let mut conditions = Vec::new();
        let mut bind_count = 0;

        // Add filtering conditions
        if let Some(session_id) = session_id {
            bind_count += 1;
            conditions.push(format!("m.session_id = ${}", bind_count));
        }

        if let Some(group_id) = group_id {
            bind_count += 1;
            conditions.push(format!("m.group_id = ${}", bind_count));
        }

        // For direct messages, show messages where user is sender or recipient
        if session_id.is_none() && group_id.is_none() {
            bind_count += 1;
            conditions.push(format!("(m.sender_id = ${} OR m.recipient_id = ${})", bind_count, bind_count));
        }

        // Add before_message_id condition
        if let Some(_) = before_message_id {
            bind_count += 1;
            conditions.push(format!("m.created_at < (SELECT created_at FROM messages WHERE message_id = ${})", bind_count));
        }

        // Add conditions to query
        if !conditions.is_empty() {
            query.push_str(" AND ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" ORDER BY m.created_at DESC");
        
        bind_count += 1;
        query.push_str(&format!(" LIMIT ${}", bind_count));

        // Build and execute query
        let mut db_query = sqlx::query_as::<_, MessageRow>(&query);

        if let Some(session_id) = session_id {
            db_query = db_query.bind(session_id);
        }

        if let Some(group_id) = group_id {
            db_query = db_query.bind(group_id);
        }

        if session_id.is_none() && group_id.is_none() {
            db_query = db_query.bind(user_id);
        }

        if let Some(before_message_id) = before_message_id {
            db_query = db_query.bind(before_message_id);
        }

        db_query = db_query.bind(limit as i64);

        let rows = db_query
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch message history: {}", e)))?;

        let messages: Vec<ChatMessageResponse> = rows
            .into_iter()
            .map(|row| ChatMessageResponse {
                message_id: row.message_id,
                sender_id: row.sender_id,
                sender_username: row.sender_username,
                content: row.content,
                recipient_id: row.recipient_id,
                session_id: row.session_id,
                group_id: row.group_id,
                message_type: row.message_type,
                moderation_status: row.moderation_status,
                timestamp: row.created_at,
                edited_at: row.updated_at,
                is_edited: row.is_edited,
            })
            .collect();

        // Check if there are more messages
        let has_more = messages.len() == limit as usize;

        Ok(MessageHistoryResponse {
            messages,
            has_more,
            total_count: None, // Could be implemented if needed
        })
    }

    pub async fn update_message(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        new_content: String,
    ) -> Result<ChatMessageResponse, AppError> {
        // Validate message content
        if new_content.trim().is_empty() {
            return Err(AppError::BadRequest("Message content cannot be empty".to_string()));
        }

        // Perform content moderation
        let moderation_status = self.moderate_content(&new_content).await?;

        // Update message in database
        let query = r#"
            UPDATE messages 
            SET content = $1, moderation_status = $2, updated_at = $3, is_edited = true
            WHERE message_id = $4 AND sender_id = $5
            RETURNING 
                message_id, sender_id, recipient_id, session_id, group_id,
                content, message_type, moderation_status, created_at, updated_at, is_edited
        "#;

        let updated_at = Utc::now();
        let row = sqlx::query_as::<_, MessageUpdateRow>(query)
            .bind(&new_content)
            .bind(&moderation_status)
            .bind(updated_at)
            .bind(message_id)
            .bind(user_id)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update message: {}", e)))?;

        let row = row.ok_or_else(|| {
            AppError::NotFound("Message not found or you don't have permission to edit it".to_string())
        })?;

        // Get sender information
        let sender_info = self.get_user_info(user_id).await?;

        Ok(ChatMessageResponse {
            message_id: row.message_id,
            sender_id: row.sender_id,
            sender_username: sender_info.username,
            content: row.content,
            recipient_id: row.recipient_id,
            session_id: row.session_id,
            group_id: row.group_id,
            message_type: row.message_type,
            moderation_status: row.moderation_status,
            timestamp: row.created_at,
            edited_at: Some(row.updated_at),
            is_edited: row.is_edited,
        })
    }

    pub async fn delete_message(
        &self,
        message_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let query = r#"
            UPDATE messages 
            SET content = '[Message deleted]', is_deleted = true, updated_at = $1
            WHERE message_id = $2 AND sender_id = $3
        "#;

        let result = sqlx::query(query)
            .bind(Utc::now())
            .bind(message_id)
            .bind(user_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete message: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Message not found or you don't have permission to delete it".to_string()
            ));
        }

        Ok(())
    }

    // Private helper methods

    async fn check_rate_limit(&self, user_id: Uuid) -> Result<(), AppError> {
        let key = format!("rate_limit:messages:{}", user_id);
        let current_count: i32 = self.redis_service
            .get(&key)
            .await
            .unwrap_or(0);

        if current_count >= 60 { // 60 messages per minute
            return Err(AppError::TooManyRequests(
                "Message rate limit exceeded".to_string()
            ));
        }

        Ok(())
    }

    async fn increment_rate_limit_counter(&self, user_id: Uuid) -> Result<(), AppError> {
        let key = format!("rate_limit:messages:{}", user_id);
        let _: () = self.redis_service
            .incr_with_expiry(&key, 1, 60)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update rate limit: {}", e)))?;

        Ok(())
    }

    async fn moderate_content(&self, content: &str) -> Result<ModerationStatus, AppError> {
        // Simple content moderation - in production, use a proper moderation service
        let banned_words = ["spam", "abuse", "inappropriate"];
        
        for word in banned_words {
            if content.to_lowercase().contains(word) {
                return Ok(ModerationStatus::Flagged);
            }
        }

        Ok(ModerationStatus::Approved)
    }

    async fn get_user_info(&self, user_id: Uuid) -> Result<UserInfo, AppError> {
        let query = "SELECT username FROM users WHERE user_id = $1";
        
        let row = sqlx::query_as::<_, UserInfoRow>(query)
            .bind(user_id)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch user info: {}", e)))?;

        let row = row.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(UserInfo {
            username: row.username,
        })
    }

    async fn cache_recent_message(
        &self,
        message_id: Uuid,
        content: &str,
        sender_id: Uuid,
        timestamp: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let key = format!("recent_message:{}", message_id);
        let message_data = serde_json::json!({
            "content": content,
            "sender_id": sender_id,
            "timestamp": timestamp
        });

        let _: () = self.redis_service
            .set_with_expiry(&key, &message_data.to_string(), 3600) // Cache for 1 hour
            .await
            .map_err(|e| AppError::Internal(format!("Failed to cache message: {}", e)))?;

        Ok(())
    }

    async fn update_conversation_activity(
        &self,
        recipient_id: Option<Uuid>,
        session_id: Option<Uuid>,
        group_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        let key = if let Some(recipient_id) = recipient_id {
            format!("conversation:dm:{}", recipient_id)
        } else if let Some(session_id) = session_id {
            format!("conversation:session:{}", session_id)
        } else if let Some(group_id) = group_id {
            format!("conversation:group:{}", group_id)
        } else {
            return Ok(());
        };

        let _: () = self.redis_service
            .set_with_expiry(&key, &Utc::now().to_rfc3339(), 86400) // Cache for 24 hours
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update conversation activity: {}", e)))?;

        Ok(())
    }
}

// Database row structs
#[derive(sqlx::FromRow)]
struct MessageRow {
    message_id: Uuid,
    sender_id: Uuid,
    sender_username: String,
    content: String,
    recipient_id: Option<Uuid>,
    session_id: Option<Uuid>,
    group_id: Option<Uuid>,
    message_type: MessageType,
    moderation_status: ModerationStatus,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    is_edited: bool,
}

#[derive(sqlx::FromRow)]
struct MessageUpdateRow {
    message_id: Uuid,
    sender_id: Uuid,
    recipient_id: Option<Uuid>,
    session_id: Option<Uuid>,
    group_id: Option<Uuid>,
    content: String,
    message_type: MessageType,
    moderation_status: ModerationStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    is_edited: bool,
}

#[derive(sqlx::FromRow)]
struct UserInfoRow {
    username: String,
}

struct UserInfo {
    username: String,
}