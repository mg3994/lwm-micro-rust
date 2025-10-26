use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use redis::{AsyncCommands, Client};
use futures::StreamExt;

use linkwithmentor_common::{AppError, RedisService};
use crate::{
    models::{WSMessage, ChatRoomType},
    connection_manager::ConnectionManager,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubSubMessage {
    pub channel: String,
    pub message_type: PubSubMessageType,
    pub payload: serde_json::Value,
    pub sender_instance: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PubSubMessageType {
    ChatMessage {
        message_id: Uuid,
        sender_id: Uuid,
        recipient_id: Option<Uuid>,
        session_id: Option<Uuid>,
        group_id: Option<Uuid>,
        content: String,
        message_type: linkwithmentor_common::MessageType,
    },
    UserPresence {
        user_id: Uuid,
        username: String,
        is_online: bool,
    },
    TypingIndicator {
        user_id: Uuid,
        username: String,
        room_id: String,
        is_typing: bool,
    },
    UserJoinedRoom {
        user_id: Uuid,
        username: String,
        room_id: String,
        room_type: ChatRoomType,
    },
    UserLeftRoom {
        user_id: Uuid,
        username: String,
        room_id: String,
    },
    MessageDelivered {
        message_id: Uuid,
        recipient_id: Uuid,
    },
    MessageRead {
        message_id: Uuid,
        reader_id: Uuid,
    },
}

#[derive(Clone)]
pub struct ChatPubSub {
    redis_service: RedisService,
    connection_manager: ConnectionManager,
    instance_id: String,
    subscriber_client: Arc<RwLock<Option<redis::aio::Connection>>>,
    publisher_client: Arc<RwLock<Option<redis::aio::Connection>>>,
}

impl ChatPubSub {
    pub fn new(
        redis_service: RedisService,
        connection_manager: ConnectionManager,
    ) -> Self {
        let instance_id = format!("chat-{}", Uuid::new_v4());
        
        Self {
            redis_service,
            connection_manager,
            instance_id,
            subscriber_client: Arc::new(RwLock::new(None)),
            publisher_client: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn initialize(&self) -> Result<(), AppError> {
        // Initialize subscriber connection
        let subscriber_conn = self.redis_service.get_connection().await
            .map_err(|e| AppError::Internal(format!("Failed to create subscriber connection: {}", e)))?;
        
        *self.subscriber_client.write().await = Some(subscriber_conn);

        // Initialize publisher connection
        let publisher_conn = self.redis_service.get_connection().await
            .map_err(|e| AppError::Internal(format!("Failed to create publisher connection: {}", e)))?;
        
        *self.publisher_client.write().await = Some(publisher_conn);

        // Start listening to channels
        self.start_listening().await?;

        tracing::info!("Chat PubSub initialized with instance ID: {}", self.instance_id);
        Ok(())
    }

    async fn start_listening(&self) -> Result<(), AppError> {
        let connection_manager = self.connection_manager.clone();
        let instance_id = self.instance_id.clone();
        let redis_service = self.redis_service.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::listen_to_channels(
                redis_service,
                connection_manager,
                instance_id,
            ).await {
                tracing::error!("PubSub listener error: {}", e);
            }
        });

        Ok(())
    }

    async fn listen_to_channels(
        redis_service: RedisService,
        connection_manager: ConnectionManager,
        instance_id: String,
    ) -> Result<(), AppError> {
        let mut conn = redis_service.get_connection().await
            .map_err(|e| AppError::Internal(format!("Failed to get Redis connection: {}", e)))?;

        // Subscribe to channels
        let channels = vec![
            "chat:messages",
            "chat:presence",
            "chat:typing",
            "chat:rooms",
            "chat:delivery",
        ];

        let mut pubsub = conn.as_mut().into_pubsub();
        
        for channel in &channels {
            pubsub.subscribe(channel).await
                .map_err(|e| AppError::Internal(format!("Failed to subscribe to {}: {}", channel, e)))?;
        }

        tracing::info!("Subscribed to PubSub channels: {:?}", channels);

        let mut stream = pubsub.on_message();
        
        while let Some(msg) = stream.next().await {
            if let Ok(payload) = msg.get_payload::<String>() {
                if let Err(e) = Self::handle_pubsub_message(
                    &payload,
                    &connection_manager,
                    &instance_id,
                ).await {
                    tracing::error!("Error handling PubSub message: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn handle_pubsub_message(
        payload: &str,
        connection_manager: &ConnectionManager,
        instance_id: &str,
    ) -> Result<(), AppError> {
        let pubsub_msg: PubSubMessage = serde_json::from_str(payload)
            .map_err(|e| AppError::Internal(format!("Failed to parse PubSub message: {}", e)))?;

        // Skip messages from the same instance to avoid loops
        if pubsub_msg.sender_instance == instance_id {
            return Ok(());
        }

        match pubsub_msg.message_type {
            PubSubMessageType::ChatMessage {
                message_id,
                sender_id,
                recipient_id,
                session_id,
                group_id,
                content,
                message_type,
            } => {
                let ws_message = WSMessage::MessageReceived {
                    message_id,
                    sender_id,
                    content,
                    recipient_id,
                    session_id,
                    group_id,
                    message_type,
                    timestamp: pubsub_msg.timestamp,
                    moderation_status: linkwithmentor_common::ModerationStatus::Approved,
                };

                // Route message to appropriate recipients
                if let Some(recipient_id) = recipient_id {
                    // Direct message
                    connection_manager.send_to_user(recipient_id, ws_message).await?;
                } else if let Some(session_id) = session_id {
                    // Session message
                    let room_id = format!("session_{}", session_id);
                    connection_manager.send_to_room(&room_id, ws_message, Some(sender_id)).await?;
                } else if let Some(group_id) = group_id {
                    // Group message
                    let room_id = format!("group_{}", group_id);
                    connection_manager.send_to_room(&room_id, ws_message, Some(sender_id)).await?;
                }
            }

            PubSubMessageType::UserPresence { user_id, username, is_online } => {
                let ws_message = if is_online {
                    WSMessage::UserJoined {
                        user_id,
                        username,
                        session_id: None,
                        group_id: None,
                    }
                } else {
                    WSMessage::UserLeft {
                        user_id,
                        username,
                        session_id: None,
                        group_id: None,
                    }
                };

                // Broadcast to all connected users
                let online_users = connection_manager.get_online_users().await;
                for online_user_id in online_users {
                    if online_user_id != user_id {
                        let _ = connection_manager.send_to_user(online_user_id, ws_message.clone()).await;
                    }
                }
            }

            PubSubMessageType::TypingIndicator { user_id, username, room_id, is_typing } => {
                let ws_message = WSMessage::TypingIndicator {
                    user_id,
                    username,
                    is_typing,
                    session_id: if room_id.starts_with("session_") {
                        room_id.strip_prefix("session_").and_then(|s| s.parse().ok())
                    } else {
                        None
                    },
                    group_id: if room_id.starts_with("group_") {
                        room_id.strip_prefix("group_").and_then(|s| s.parse().ok())
                    } else {
                        None
                    },
                };

                connection_manager.send_to_room(&room_id, ws_message, Some(user_id)).await?;
            }

            PubSubMessageType::UserJoinedRoom { user_id, username, room_id, .. } => {
                let ws_message = WSMessage::UserJoined {
                    user_id,
                    username,
                    session_id: if room_id.starts_with("session_") {
                        room_id.strip_prefix("session_").and_then(|s| s.parse().ok())
                    } else {
                        None
                    },
                    group_id: if room_id.starts_with("group_") {
                        room_id.strip_prefix("group_").and_then(|s| s.parse().ok())
                    } else {
                        None
                    },
                };

                connection_manager.send_to_room(&room_id, ws_message, Some(user_id)).await?;
            }

            PubSubMessageType::UserLeftRoom { user_id, username, room_id } => {
                let ws_message = WSMessage::UserLeft {
                    user_id,
                    username,
                    session_id: if room_id.starts_with("session_") {
                        room_id.strip_prefix("session_").and_then(|s| s.parse().ok())
                    } else {
                        None
                    },
                    group_id: if room_id.starts_with("group_") {
                        room_id.strip_prefix("group_").and_then(|s| s.parse().ok())
                    } else {
                        None
                    },
                };

                connection_manager.send_to_room(&room_id, ws_message, Some(user_id)).await?;
            }

            PubSubMessageType::MessageDelivered { message_id, recipient_id } => {
                let ws_message = WSMessage::Ack { message_id };
                connection_manager.send_to_user(recipient_id, ws_message).await?;
            }

            PubSubMessageType::MessageRead { message_id, reader_id } => {
                let ws_message = WSMessage::Ack { message_id };
                connection_manager.send_to_user(reader_id, ws_message).await?;
            }
        }

        Ok(())
    }

    // Publish methods

    pub async fn publish_chat_message(
        &self,
        message_id: Uuid,
        sender_id: Uuid,
        recipient_id: Option<Uuid>,
        session_id: Option<Uuid>,
        group_id: Option<Uuid>,
        content: String,
        message_type: linkwithmentor_common::MessageType,
    ) -> Result<(), AppError> {
        let pubsub_msg = PubSubMessage {
            channel: "chat:messages".to_string(),
            message_type: PubSubMessageType::ChatMessage {
                message_id,
                sender_id,
                recipient_id,
                session_id,
                group_id,
                content,
                message_type,
            },
            payload: serde_json::Value::Null,
            sender_instance: self.instance_id.clone(),
            timestamp: chrono::Utc::now(),
        };

        self.publish("chat:messages", &pubsub_msg).await
    }

    pub async fn publish_user_presence(
        &self,
        user_id: Uuid,
        username: String,
        is_online: bool,
    ) -> Result<(), AppError> {
        let pubsub_msg = PubSubMessage {
            channel: "chat:presence".to_string(),
            message_type: PubSubMessageType::UserPresence {
                user_id,
                username,
                is_online,
            },
            payload: serde_json::Value::Null,
            sender_instance: self.instance_id.clone(),
            timestamp: chrono::Utc::now(),
        };

        self.publish("chat:presence", &pubsub_msg).await
    }

    pub async fn publish_typing_indicator(
        &self,
        user_id: Uuid,
        username: String,
        room_id: String,
        is_typing: bool,
    ) -> Result<(), AppError> {
        let pubsub_msg = PubSubMessage {
            channel: "chat:typing".to_string(),
            message_type: PubSubMessageType::TypingIndicator {
                user_id,
                username,
                room_id,
                is_typing,
            },
            payload: serde_json::Value::Null,
            sender_instance: self.instance_id.clone(),
            timestamp: chrono::Utc::now(),
        };

        self.publish("chat:typing", &pubsub_msg).await
    }

    pub async fn publish_user_joined_room(
        &self,
        user_id: Uuid,
        username: String,
        room_id: String,
        room_type: ChatRoomType,
    ) -> Result<(), AppError> {
        let pubsub_msg = PubSubMessage {
            channel: "chat:rooms".to_string(),
            message_type: PubSubMessageType::UserJoinedRoom {
                user_id,
                username,
                room_id,
                room_type,
            },
            payload: serde_json::Value::Null,
            sender_instance: self.instance_id.clone(),
            timestamp: chrono::Utc::now(),
        };

        self.publish("chat:rooms", &pubsub_msg).await
    }

    pub async fn publish_user_left_room(
        &self,
        user_id: Uuid,
        username: String,
        room_id: String,
    ) -> Result<(), AppError> {
        let pubsub_msg = PubSubMessage {
            channel: "chat:rooms".to_string(),
            message_type: PubSubMessageType::UserLeftRoom {
                user_id,
                username,
                room_id,
            },
            payload: serde_json::Value::Null,
            sender_instance: self.instance_id.clone(),
            timestamp: chrono::Utc::now(),
        };

        self.publish("chat:rooms", &pubsub_msg).await
    }

    pub async fn publish_message_delivered(
        &self,
        message_id: Uuid,
        recipient_id: Uuid,
    ) -> Result<(), AppError> {
        let pubsub_msg = PubSubMessage {
            channel: "chat:delivery".to_string(),
            message_type: PubSubMessageType::MessageDelivered {
                message_id,
                recipient_id,
            },
            payload: serde_json::Value::Null,
            sender_instance: self.instance_id.clone(),
            timestamp: chrono::Utc::now(),
        };

        self.publish("chat:delivery", &pubsub_msg).await
    }

    async fn publish(&self, channel: &str, message: &PubSubMessage) -> Result<(), AppError> {
        let payload = serde_json::to_string(message)
            .map_err(|e| AppError::Internal(format!("Failed to serialize PubSub message: {}", e)))?;

        let mut conn_guard = self.publisher_client.write().await;
        if let Some(conn) = conn_guard.as_mut() {
            let _: () = conn.publish(channel, payload).await
                .map_err(|e| AppError::Internal(format!("Failed to publish to Redis: {}", e)))?;
        } else {
            return Err(AppError::Internal("Publisher connection not initialized".to_string()));
        }

        Ok(())
    }

    // Queue messages for offline users
    pub async fn queue_message_for_offline_user(
        &self,
        user_id: Uuid,
        message: &WSMessage,
    ) -> Result<(), AppError> {
        let queue_key = format!("offline_messages:{}", user_id);
        let message_json = serde_json::to_string(message)
            .map_err(|e| AppError::Internal(format!("Failed to serialize message: {}", e)))?;

        // Add to Redis list with expiration
        let _: () = self.redis_service
            .lpush(&queue_key, &message_json)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to queue message: {}", e)))?;

        // Set expiration for the queue (7 days)
        let _: () = self.redis_service
            .expire(&queue_key, 604800)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set queue expiration: {}", e)))?;

        Ok(())
    }

    // Retrieve queued messages for user when they come online
    pub async fn get_queued_messages(&self, user_id: Uuid) -> Result<Vec<WSMessage>, AppError> {
        let queue_key = format!("offline_messages:{}", user_id);
        
        let messages: Vec<String> = self.redis_service
            .lrange(&queue_key, 0, -1)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get queued messages: {}", e)))?;

        // Clear the queue after retrieving
        let _: () = self.redis_service
            .del(&queue_key)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to clear message queue: {}", e)))?;

        let mut ws_messages = Vec::new();
        for message_json in messages {
            if let Ok(ws_message) = serde_json::from_str::<WSMessage>(&message_json) {
                ws_messages.push(ws_message);
            }
        }

        Ok(ws_messages)
    }

    // Clean up expired typing indicators
    pub async fn cleanup_expired_typing_indicators(&self) -> Result<(), AppError> {
        let pattern = "typing:*";
        let keys: Vec<String> = self.redis_service
            .keys(&pattern)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get typing indicator keys: {}", e)))?;

        let now = chrono::Utc::now().timestamp();
        
        for key in keys {
            let expires_at: Option<i64> = self.redis_service
                .hget(&key, "expires_at")
                .await
                .unwrap_or(None);

            if let Some(expires_at) = expires_at {
                if expires_at < now {
                    let _: () = self.redis_service
                        .del(&key)
                        .await
                        .map_err(|e| AppError::Internal(format!("Failed to delete expired typing indicator: {}", e)))?;
                }
            }
        }

        Ok(())
    }
}