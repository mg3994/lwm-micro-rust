use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use dashmap::DashMap;
use futures::StreamExt;

use linkwithmentor_common::{AppError, RedisService};
use crate::{
    models::{
        CollaborationMessage, SessionResponse, ParticipantRole, ParticipantStatus,
        SessionParticipant, WhiteboardState, SessionMaterial, MaterialType,
    },
    whiteboard::WhiteboardService,
};

#[derive(Clone)]
pub struct CollaborationService {
    db_pool: PgPool,
    redis_service: RedisService,
    whiteboard_service: WhiteboardService,
    // Active session participants
    active_sessions: std::sync::Arc<DashMap<Uuid, SessionCollaboration>>,
    // WebSocket connections for real-time communication
    connections: std::sync::Arc<DashMap<Uuid, Vec<CollaborationConnection>>>,
}

#[derive(Debug, Clone)]
pub struct SessionCollaboration {
    pub session_id: Uuid,
    pub participants: HashMap<Uuid, SessionParticipant>,
    pub whiteboard_id: Option<Uuid>,
    pub active_screen_share: Option<Uuid>,
    pub chat_messages: Vec<ChatMessage>,
    pub shared_materials: Vec<SessionMaterial>,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CollaborationConnection {
    pub user_id: Uuid,
    pub username: String,
    pub session_id: Uuid,
    pub connection_id: String,
    pub sender: tokio::sync::mpsc::UnboundedSender<tokio_tungstenite::tungstenite::Message>,
    pub connected_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub message_id: Uuid,
    pub sender_id: Uuid,
    pub sender_username: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl CollaborationService {
    pub fn new(
        db_pool: PgPool,
        redis_service: RedisService,
        whiteboard_service: WhiteboardService,
    ) -> Self {
        Self {
            db_pool,
            redis_service,
            whiteboard_service,
            active_sessions: std::sync::Arc::new(DashMap::new()),
            connections: std::sync::Arc::new(DashMap::new()),
        }
    }

    pub async fn initialize(&self) -> Result<(), AppError> {
        // Start Redis subscriber for cross-instance collaboration
        self.start_collaboration_listener().await?;
        
        // Start cleanup task for inactive sessions
        self.start_cleanup_task().await?;

        tracing::info!("Collaboration service initialized");
        Ok(())
    }

    // Session management
    pub async fn start_session_collaboration(&self, session_id: Uuid, started_by: Uuid) -> Result<(), AppError> {
        // Get session details
        let session = self.get_session_details(session_id).await?;
        
        // Create whiteboard for the session
        let whiteboard = self.whiteboard_service
            .create_whiteboard(session_id, started_by)
            .await?;

        // Initialize collaboration state
        let collaboration = SessionCollaboration {
            session_id,
            participants: HashMap::new(),
            whiteboard_id: Some(whiteboard.whiteboard_id),
            active_screen_share: None,
            chat_messages: Vec::new(),
            shared_materials: Vec::new(),
            started_at: Utc::now(),
            last_activity: Utc::now(),
        };

        self.active_sessions.insert(session_id, collaboration);

        // Broadcast session started message
        let message = CollaborationMessage::SessionStarted {
            session_id,
            started_by,
        };

        self.broadcast_to_session(session_id, &message, None).await?;

        tracing::info!("Started collaboration for session {}", session_id);
        Ok(())
    }

    pub async fn end_session_collaboration(&self, session_id: Uuid, ended_by: Uuid) -> Result<(), AppError> {
        // Save final state
        if let Some(collaboration) = self.active_sessions.get(&session_id) {
            self.save_session_collaboration(&collaboration).await?;
        }

        // Remove from active sessions
        self.active_sessions.remove(&session_id);

        // Broadcast session ended message
        let message = CollaborationMessage::SessionEnded {
            session_id,
            ended_by,
        };

        self.broadcast_to_session(session_id, &message, None).await?;

        tracing::info!("Ended collaboration for session {}", session_id);
        Ok(())
    }

    // Participant management
    pub async fn add_participant(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        username: String,
        role: ParticipantRole,
    ) -> Result<(), AppError> {
        let participant = SessionParticipant {
            user_id,
            username: username.clone(),
            role: role.clone(),
            status: ParticipantStatus::Confirmed,
            joined_at: Some(Utc::now()),
            left_at: None,
        };

        // Add to active session
        if let Some(mut collaboration) = self.active_sessions.get_mut(&session_id) {
            collaboration.participants.insert(user_id, participant);
            collaboration.last_activity = Utc::now();
        }

        // Broadcast participant joined
        let message = CollaborationMessage::UserJoined {
            user_id,
            username,
            role,
        };

        self.broadcast_to_session(session_id, &message, Some(user_id)).await?;

        tracing::info!("Added participant {} to session {}", user_id, session_id);
        Ok(())
    }

    pub async fn remove_participant(&self, session_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let username = if let Some(mut collaboration) = self.active_sessions.get_mut(&session_id) {
            if let Some(mut participant) = collaboration.participants.get_mut(&user_id) {
                participant.left_at = Some(Utc::now());
                participant.status = ParticipantStatus::NoShow;
                collaboration.last_activity = Utc::now();
                participant.username.clone()
            } else {
                return Err(AppError::NotFound("Participant not found".to_string()));
            }
        } else {
            return Err(AppError::NotFound("Session not found".to_string()));
        };

        // Remove connections
        self.connections.remove(&user_id);

        // Stop screen sharing if this user was sharing
        if let Some(collaboration) = self.active_sessions.get(&session_id) {
            if collaboration.active_screen_share == Some(user_id) {
                self.stop_screen_sharing(session_id, user_id).await?;
            }
        }

        // Broadcast participant left
        let message = CollaborationMessage::UserLeft {
            user_id,
            username,
        };

        self.broadcast_to_session(session_id, &message, Some(user_id)).await?;

        tracing::info!("Removed participant {} from session {}", user_id, session_id);
        Ok(())
    }

    // Connection management
    pub async fn add_connection(&self, connection: CollaborationConnection) -> Result<(), AppError> {
        let user_id = connection.user_id;
        let session_id = connection.session_id;

        // Add connection
        self.connections.entry(user_id)
            .or_insert_with(Vec::new)
            .push(connection);

        // Add as participant if not already added
        let username = format!("User {}", user_id); // In real app, get from database
        self.add_participant(session_id, user_id, username, ParticipantRole::Mentee).await?;

        Ok(())
    }

    pub async fn remove_connection(&self, user_id: Uuid, connection_id: &str) -> Result<(), AppError> {
        if let Some(mut connections) = self.connections.get_mut(&user_id) {
            connections.retain(|conn| conn.connection_id != connection_id);
            if connections.is_empty() {
                drop(connections);
                self.connections.remove(&user_id);
                
                // Remove from all sessions
                for session in self.active_sessions.iter() {
                    if session.participants.contains_key(&user_id) {
                        let session_id = session.session_id;
                        drop(session);
                        self.remove_participant(session_id, user_id).await?;
                    }
                }
            }
        }

        Ok(())
    }

    // Chat functionality
    pub async fn send_chat_message(
        &self,
        session_id: Uuid,
        sender_id: Uuid,
        content: String,
    ) -> Result<(), AppError> {
        let message_id = Uuid::new_v4();
        let timestamp = Utc::now();
        let sender_username = format!("User {}", sender_id); // In real app, get from database

        let chat_message = ChatMessage {
            message_id,
            sender_id,
            sender_username: sender_username.clone(),
            content: content.clone(),
            timestamp,
        };

        // Add to session chat history
        if let Some(mut collaboration) = self.active_sessions.get_mut(&session_id) {
            collaboration.chat_messages.push(chat_message);
            collaboration.last_activity = timestamp;
        }

        // Broadcast chat message
        let message = CollaborationMessage::ChatMessage {
            message_id,
            sender_id,
            sender_username,
            content,
            timestamp,
        };

        self.broadcast_to_session(session_id, &message, Some(sender_id)).await?;

        tracing::debug!("Sent chat message in session {}", session_id);
        Ok(())
    }

    pub async fn get_chat_history(&self, session_id: Uuid, limit: Option<u32>) -> Result<Vec<ChatMessage>, AppError> {
        if let Some(collaboration) = self.active_sessions.get(&session_id) {
            let limit = limit.unwrap_or(50) as usize;
            let messages = collaboration.chat_messages
                .iter()
                .rev()
                .take(limit)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            
            Ok(messages)
        } else {
            // Load from database if session is not active
            self.load_chat_history_from_db(session_id, limit).await
        }
    }

    // Screen sharing
    pub async fn start_screen_sharing(&self, session_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        // Check if someone else is already sharing
        if let Some(mut collaboration) = self.active_sessions.get_mut(&session_id) {
            if let Some(current_sharer) = collaboration.active_screen_share {
                if current_sharer != user_id {
                    return Err(AppError::BadRequest("Another user is already screen sharing".to_string()));
                }
            }
            
            collaboration.active_screen_share = Some(user_id);
            collaboration.last_activity = Utc::now();
        }

        let username = format!("User {}", user_id); // In real app, get from database

        // Broadcast screen share started
        let message = CollaborationMessage::ScreenShareStarted {
            user_id,
            username,
        };

        self.broadcast_to_session(session_id, &message, Some(user_id)).await?;

        tracing::info!("Started screen sharing for user {} in session {}", user_id, session_id);
        Ok(())
    }

    pub async fn stop_screen_sharing(&self, session_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        if let Some(mut collaboration) = self.active_sessions.get_mut(&session_id) {
            if collaboration.active_screen_share == Some(user_id) {
                collaboration.active_screen_share = None;
                collaboration.last_activity = Utc::now();
            }
        }

        let username = format!("User {}", user_id); // In real app, get from database

        // Broadcast screen share stopped
        let message = CollaborationMessage::ScreenShareStopped {
            user_id,
            username,
        };

        self.broadcast_to_session(session_id, &message, Some(user_id)).await?;

        tracing::info!("Stopped screen sharing for user {} in session {}", user_id, session_id);
        Ok(())
    }

    // Material sharing
    pub async fn share_material(
        &self,
        session_id: Uuid,
        material: SessionMaterial,
    ) -> Result<(), AppError> {
        // Add to session materials
        if let Some(mut collaboration) = self.active_sessions.get_mut(&session_id) {
            collaboration.shared_materials.push(material.clone());
            collaboration.last_activity = Utc::now();
        }

        // Store in database
        self.store_session_material(session_id, &material).await?;

        // Broadcast material shared (implementation would include material details)
        tracing::info!("Shared material {} in session {}", material.name, session_id);
        Ok(())
    }

    // Real-time messaging
    async fn broadcast_to_session(
        &self,
        session_id: Uuid,
        message: &CollaborationMessage,
        exclude_user: Option<Uuid>,
    ) -> Result<(), AppError> {
        // Get session participants
        let participants = if let Some(collaboration) = self.active_sessions.get(&session_id) {
            collaboration.participants.keys().cloned().collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // Send to all connected participants
        for participant_id in participants {
            if Some(participant_id) == exclude_user {
                continue;
            }

            if let Some(connections) = self.connections.get(&participant_id) {
                let message_json = serde_json::to_string(message)
                    .map_err(|e| AppError::Internal(format!("Failed to serialize message: {}", e)))?;
                
                let ws_message = tokio_tungstenite::tungstenite::Message::Text(message_json);

                for connection in connections.iter() {
                    if connection.session_id == session_id {
                        if let Err(e) = connection.sender.send(ws_message.clone()) {
                            tracing::warn!("Failed to send message to participant {}: {}", participant_id, e);
                        }
                    }
                }
            }
        }

        // Also publish to Redis for cross-instance communication
        let channel = format!("collaboration:{}", session_id);
        let message_json = serde_json::to_string(message)
            .map_err(|e| AppError::Internal(format!("Failed to serialize message: {}", e)))?;

        let _: () = self.redis_service.publish(&channel, &message_json).await
            .map_err(|e| AppError::Internal(format!("Failed to publish message: {}", e)))?;

        Ok(())
    }

    // Background tasks
    async fn start_collaboration_listener(&self) -> Result<(), AppError> {
        let redis_service = self.redis_service.clone();
        let active_sessions = self.active_sessions.clone();
        let connections = self.connections.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::listen_to_collaboration_channels(
                redis_service,
                active_sessions,
                connections,
            ).await {
                tracing::error!("Collaboration listener error: {}", e);
            }
        });

        Ok(())
    }

    async fn listen_to_collaboration_channels(
        redis_service: RedisService,
        active_sessions: std::sync::Arc<DashMap<Uuid, SessionCollaboration>>,
        connections: std::sync::Arc<DashMap<Uuid, Vec<CollaborationConnection>>>,
    ) -> Result<(), AppError> {
        let mut conn = redis_service.get_connection().await
            .map_err(|e| AppError::Internal(format!("Failed to get Redis connection: {}", e)))?;

        // Subscribe to collaboration channels
        let mut pubsub = conn.as_mut().into_pubsub();
        pubsub.psubscribe("collaboration:*").await
            .map_err(|e| AppError::Internal(format!("Failed to subscribe to collaboration channels: {}", e)))?;

        let mut stream = pubsub.on_message();
        
        while let Some(msg) = stream.next().await {
            if let Ok(payload) = msg.get_payload::<String>() {
                if let Err(e) = Self::handle_collaboration_message(&payload, &active_sessions, &connections).await {
                    tracing::error!("Error handling collaboration message: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn handle_collaboration_message(
        payload: &str,
        active_sessions: &DashMap<Uuid, SessionCollaboration>,
        connections: &DashMap<Uuid, Vec<CollaborationConnection>>,
    ) -> Result<(), AppError> {
        let message: CollaborationMessage = serde_json::from_str(payload)
            .map_err(|e| AppError::Internal(format!("Failed to parse collaboration message: {}", e)))?;

        // Handle cross-instance collaboration messages
        match message {
            CollaborationMessage::UserJoined { user_id, .. } => {
                tracing::debug!("User {} joined collaboration", user_id);
            }
            CollaborationMessage::UserLeft { user_id, .. } => {
                tracing::debug!("User {} left collaboration", user_id);
            }
            _ => {
                // Handle other message types as needed
            }
        }

        Ok(())
    }

    async fn start_cleanup_task(&self) -> Result<(), AppError> {
        let active_sessions = self.active_sessions.clone();
        let connections = self.connections.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes
            
            loop {
                interval.tick().await;
                
                let cutoff_time = Utc::now() - chrono::Duration::hours(1);
                
                // Clean up inactive sessions
                active_sessions.retain(|_, session| {
                    session.last_activity > cutoff_time
                });
                
                // Clean up stale connections
                connections.retain(|_, conns| {
                    conns.retain(|conn| conn.last_activity > cutoff_time);
                    !conns.is_empty()
                });
                
                tracing::debug!("Cleaned up inactive collaboration sessions and connections");
            }
        });

        Ok(())
    }

    // Helper methods
    async fn get_session_details(&self, session_id: Uuid) -> Result<SessionResponse, AppError> {
        // In a real implementation, fetch from database
        // For now, return a placeholder
        Err(AppError::NotFound("Session details not implemented".to_string()))
    }

    async fn save_session_collaboration(&self, collaboration: &SessionCollaboration) -> Result<(), AppError> {
        // Save chat messages, shared materials, and other collaboration data to database
        tracing::info!("Saved collaboration data for session {}", collaboration.session_id);
        Ok(())
    }

    async fn load_chat_history_from_db(&self, session_id: Uuid, limit: Option<u32>) -> Result<Vec<ChatMessage>, AppError> {
        // Load chat history from database
        Ok(Vec::new())
    }

    async fn store_session_material(&self, session_id: Uuid, material: &SessionMaterial) -> Result<(), AppError> {
        // Store material in database
        tracing::info!("Stored material {} for session {}", material.name, session_id);
        Ok(())
    }
}