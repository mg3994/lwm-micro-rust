use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

use crate::models::{WSMessage, ConnectionInfo, ChatRoom, ChatRoomType};
use linkwithmentor_common::AppError;

// Type alias for WebSocket sender
pub type WSSender = mpsc::UnboundedSender<Message>;

#[derive(Clone)]
pub struct ConnectionManager {
    // User connections: user_id -> list of connection senders
    connections: Arc<DashMap<Uuid, Vec<WSSender>>>,
    // Connection info: connection_id -> connection info
    connection_info: Arc<DashMap<String, ConnectionInfo>>,
    // Chat rooms: room_id -> room info
    chat_rooms: Arc<DashMap<String, ChatRoom>>,
    // User presence: user_id -> last activity
    user_presence: Arc<DashMap<Uuid, DateTime<Utc>>>,
    // Typing indicators: room_id -> set of typing users
    typing_indicators: Arc<DashMap<String, Vec<Uuid>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            connection_info: Arc::new(DashMap::new()),
            chat_rooms: Arc::new(DashMap::new()),
            user_presence: Arc::new(DashMap::new()),
            typing_indicators: Arc::new(DashMap::new()),
        }
    }

    // Add a new WebSocket connection
    pub async fn add_connection(
        &self,
        connection_id: String,
        user_id: Uuid,
        username: String,
        sender: WSSender,
    ) -> Result<(), AppError> {
        // Add to connections map
        self.connections.entry(user_id)
            .or_insert_with(Vec::new)
            .push(sender);

        // Store connection info
        let connection_info = ConnectionInfo {
            user_id,
            username: username.clone(),
            session_id: None,
            group_ids: Vec::new(),
            connected_at: Utc::now(),
            last_activity: Utc::now(),
        };
        
        self.connection_info.insert(connection_id.clone(), connection_info);
        
        // Update user presence
        self.user_presence.insert(user_id, Utc::now());

        tracing::info!("User {} connected with connection {}", username, connection_id);
        
        // Notify other users about user coming online
        self.broadcast_user_status_change(user_id, username, true).await;

        Ok(())
    }

    // Remove a WebSocket connection
    pub async fn remove_connection(&self, connection_id: &str) -> Result<(), AppError> {
        if let Some((_, connection_info)) = self.connection_info.remove(connection_id) {
            let user_id = connection_info.user_id;
            let username = connection_info.username.clone();

            // Remove from connections map
            if let Some(mut connections) = self.connections.get_mut(&user_id) {
                // In a real implementation, you'd need to identify which sender to remove
                // For now, we'll clear all connections for this user when any connection closes
                connections.clear();
            }

            // Check if user has any remaining connections
            let has_other_connections = self.connections.get(&user_id)
                .map(|conns| !conns.is_empty())
                .unwrap_or(false);

            if !has_other_connections {
                // User is completely offline
                self.user_presence.insert(user_id, Utc::now());
                
                // Remove from all typing indicators
                for mut typing_users in self.typing_indicators.iter_mut() {
                    typing_users.retain(|&id| id != user_id);
                }

                // Notify other users about user going offline
                self.broadcast_user_status_change(user_id, username, false).await;
            }

            tracing::info!("Connection {} removed for user {}", connection_id, user_id);
        }

        Ok(())
    }

    // Send message to a specific user
    pub async fn send_to_user(&self, user_id: Uuid, message: WSMessage) -> Result<(), AppError> {
        let message_json = serde_json::to_string(&message)
            .map_err(|e| AppError::Internal(format!("Failed to serialize message: {}", e)))?;
        
        let ws_message = Message::Text(message_json);

        if let Some(connections) = self.connections.get(&user_id) {
            let mut failed_connections = Vec::new();
            
            for (index, sender) in connections.iter().enumerate() {
                if sender.send(ws_message.clone()).is_err() {
                    failed_connections.push(index);
                }
            }
            
            // Remove failed connections
            if !failed_connections.is_empty() {
                drop(connections);
                if let Some(mut connections) = self.connections.get_mut(&user_id) {
                    for &index in failed_connections.iter().rev() {
                        if index < connections.len() {
                            connections.remove(index);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // Broadcast message to multiple users
    pub async fn broadcast_to_users(&self, user_ids: &[Uuid], message: WSMessage) -> Result<(), AppError> {
        for &user_id in user_ids {
            self.send_to_user(user_id, message.clone()).await?;
        }
        Ok(())
    }

    // Send message to all users in a chat room
    pub async fn send_to_room(&self, room_id: &str, message: WSMessage, exclude_user: Option<Uuid>) -> Result<(), AppError> {
        if let Some(room) = self.chat_rooms.get(room_id) {
            let participants: Vec<Uuid> = room.participants.iter()
                .filter(|&&user_id| Some(user_id) != exclude_user)
                .copied()
                .collect();
            
            self.broadcast_to_users(&participants, message).await?;
        }
        
        Ok(())
    }

    // Join a chat room
    pub async fn join_room(&self, user_id: Uuid, room_id: String, room_type: ChatRoomType) -> Result<(), AppError> {
        // Create or get existing room
        let room = self.chat_rooms.entry(room_id.clone()).or_insert_with(|| {
            ChatRoom {
                room_id: room_id.clone(),
                room_type,
                participants: vec![user_id],
                created_at: Utc::now(),
                last_activity: Utc::now(),
            }
        });

        // Add user to room if not already present
        if !room.participants.contains(&user_id) {
            room.participants.push(user_id);
        }

        // Update connection info
        for mut conn_info in self.connection_info.iter_mut() {
            if conn_info.user_id == user_id {
                if let Ok(room_uuid) = room_id.parse::<Uuid>() {
                    if !conn_info.group_ids.contains(&room_uuid) {
                        conn_info.group_ids.push(room_uuid);
                    }
                }
            }
        }

        tracing::info!("User {} joined room {}", user_id, room_id);
        Ok(())
    }

    // Leave a chat room
    pub async fn leave_room(&self, user_id: Uuid, room_id: &str) -> Result<(), AppError> {
        if let Some(mut room) = self.chat_rooms.get_mut(room_id) {
            room.participants.retain(|&id| id != user_id);
            
            // Remove empty rooms
            if room.participants.is_empty() {
                drop(room);
                self.chat_rooms.remove(room_id);
            }
        }

        // Update connection info
        for mut conn_info in self.connection_info.iter_mut() {
            if conn_info.user_id == user_id {
                if let Ok(room_uuid) = room_id.parse::<Uuid>() {
                    conn_info.group_ids.retain(|&id| id != room_uuid);
                }
            }
        }

        tracing::info!("User {} left room {}", user_id, room_id);
        Ok(())
    }

    // Get online users
    pub async fn get_online_users(&self) -> Vec<Uuid> {
        self.connections.iter()
            .filter(|entry| !entry.value().is_empty())
            .map(|entry| *entry.key())
            .collect()
    }

    // Get users in a room
    pub async fn get_room_participants(&self, room_id: &str) -> Vec<Uuid> {
        self.chat_rooms.get(room_id)
            .map(|room| room.participants.clone())
            .unwrap_or_default()
    }

    // Update typing indicator
    pub async fn set_typing_indicator(&self, room_id: String, user_id: Uuid, is_typing: bool) -> Result<(), AppError> {
        if is_typing {
            self.typing_indicators.entry(room_id.clone())
                .or_insert_with(Vec::new)
                .push(user_id);
        } else {
            if let Some(mut typing_users) = self.typing_indicators.get_mut(&room_id) {
                typing_users.retain(|&id| id != user_id);
            }
        }

        Ok(())
    }

    // Get typing users in a room
    pub async fn get_typing_users(&self, room_id: &str) -> Vec<Uuid> {
        self.typing_indicators.get(room_id)
            .map(|users| users.clone())
            .unwrap_or_default()
    }

    // Update user activity
    pub async fn update_user_activity(&self, user_id: Uuid) {
        self.user_presence.insert(user_id, Utc::now());
        
        // Update connection info
        for mut conn_info in self.connection_info.iter_mut() {
            if conn_info.user_id == user_id {
                conn_info.last_activity = Utc::now();
            }
        }
    }

    // Get connection count for user
    pub async fn get_user_connection_count(&self, user_id: Uuid) -> usize {
        self.connections.get(&user_id)
            .map(|conns| conns.len())
            .unwrap_or(0)
    }

    // Broadcast user status change
    async fn broadcast_user_status_change(&self, user_id: Uuid, username: String, is_online: bool) {
        let message = if is_online {
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
        let online_users = self.get_online_users().await;
        for online_user_id in online_users {
            if online_user_id != user_id {
                let _ = self.send_to_user(online_user_id, message.clone()).await;
            }
        }
    }

    // Clean up inactive connections
    pub async fn cleanup_inactive_connections(&self, timeout_seconds: u64) {
        let cutoff_time = Utc::now() - chrono::Duration::seconds(timeout_seconds as i64);
        let mut inactive_connections = Vec::new();

        for conn_info in self.connection_info.iter() {
            if conn_info.last_activity < cutoff_time {
                inactive_connections.push(conn_info.key().clone());
            }
        }

        for connection_id in inactive_connections {
            let _ = self.remove_connection(&connection_id).await;
        }
    }
}