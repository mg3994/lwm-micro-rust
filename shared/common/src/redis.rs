use redis::{Client, Connection, ConnectionManager, RedisResult, AsyncCommands, Commands};
use serde::{Serialize, Deserialize};
use std::time::Duration;
use crate::{RedisConfig, AppError};

pub struct RedisService {
    manager: ConnectionManager,
    client: Client,
}

impl RedisService {
    pub async fn new(config: &RedisConfig) -> Result<Self, AppError> {
        let client = Client::open(config.connection_string())
            .map_err(|e| AppError::Redis(e))?;
        
        let manager = ConnectionManager::new(client.clone())
            .await
            .map_err(|e| AppError::Redis(e))?;
        
        // Test connection
        let mut conn = manager.clone();
        let _: String = conn.ping().await.map_err(|e| AppError::Redis(e))?;
        
        tracing::info!("Redis connection established");
        
        Ok(Self { manager, client })
    }

    pub async fn get_connection(&self) -> Result<ConnectionManager, AppError> {
        Ok(self.manager.clone())
    }

    // Session management
    pub async fn set_session(&self, user_id: &str, token: &str, expiry_seconds: u64) -> Result<(), AppError> {
        let mut conn = self.manager.clone();
        conn.set_ex(format!("session:{}", user_id), token, expiry_seconds)
            .await
            .map_err(|e| AppError::Redis(e))
    }

    pub async fn get_session(&self, user_id: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.manager.clone();
        conn.get(format!("session:{}", user_id))
            .await
            .map_err(|e| AppError::Redis(e))
    }

    pub async fn delete_session(&self, user_id: &str) -> Result<(), AppError> {
        let mut conn = self.manager.clone();
        conn.del(format!("session:{}", user_id))
            .await
            .map_err(|e| AppError::Redis(e))
    }

    // User presence
    pub async fn set_user_presence(&self, user_id: &str, status: &str, role: &str) -> Result<(), AppError> {
        let mut conn = self.manager.clone();
        let key = format!("user_presence:{}", user_id);
        
        conn.hset_multiple(&key, &[
            ("status", status),
            ("current_role", role),
            ("last_seen", &chrono::Utc::now().timestamp().to_string())
        ]).await.map_err(|e| AppError::Redis(e))?;
        
        conn.expire(&key, 300).await.map_err(|e| AppError::Redis(e))
    }

    pub async fn get_user_presence(&self, user_id: &str) -> Result<Option<UserPresence>, AppError> {
        let mut conn = self.manager.clone();
        let key = format!("user_presence:{}", user_id);
        
        let result: Vec<String> = conn.hmget(&key, &["status", "current_role", "last_seen"])
            .await
            .map_err(|e| AppError::Redis(e))?;
        
        if result.iter().all(|s| s.is_empty()) {
            return Ok(None);
        }
        
        Ok(Some(UserPresence {
            status: result[0].clone(),
            current_role: result[1].clone(),
            last_seen: result[2].parse().unwrap_or(0),
        }))
    }

    // Rate limiting
    pub async fn check_rate_limit(&self, key: &str, limit: u32, window_seconds: u64) -> Result<bool, AppError> {
        let mut conn = self.manager.clone();
        let current: u32 = conn.incr(&key, 1).await.map_err(|e| AppError::Redis(e))?;
        
        if current == 1 {
            conn.expire(&key, window_seconds as usize).await.map_err(|e| AppError::Redis(e))?;
        }
        
        Ok(current <= limit)
    }

    // Caching
    pub async fn cache_set<T>(&self, key: &str, value: &T, expiry_seconds: u64) -> Result<(), AppError>
    where
        T: Serialize,
    {
        let mut conn = self.manager.clone();
        let serialized = serde_json::to_string(value)
            .map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;
        
        conn.set_ex(key, serialized, expiry_seconds)
            .await
            .map_err(|e| AppError::Redis(e))
    }

    pub async fn cache_get<T>(&self, key: &str) -> Result<Option<T>, AppError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut conn = self.manager.clone();
        let result: Option<String> = conn.get(key).await.map_err(|e| AppError::Redis(e))?;
        
        match result {
            Some(data) => {
                let deserialized = serde_json::from_str(&data)
                    .map_err(|e| AppError::Internal(format!("Deserialization error: {}", e)))?;
                Ok(Some(deserialized))
            }
            None => Ok(None),
        }
    }

    pub async fn cache_delete(&self, key: &str) -> Result<(), AppError> {
        let mut conn = self.manager.clone();
        conn.del(key).await.map_err(|e| AppError::Redis(e))
    }

    // Pub/Sub for real-time messaging
    pub async fn publish(&self, channel: &str, message: &str) -> Result<(), AppError> {
        let mut conn = self.manager.clone();
        conn.publish(channel, message).await.map_err(|e| AppError::Redis(e))
    }

    pub async fn publish_json<T>(&self, channel: &str, message: &T) -> Result<(), AppError>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(message)
            .map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;
        self.publish(channel, &serialized).await
    }

    // Chat room management
    pub async fn add_user_to_chat_room(&self, session_id: &str, user_id: &str) -> Result<(), AppError> {
        let mut conn = self.manager.clone();
        conn.sadd(format!("chat_room:{}", session_id), user_id)
            .await
            .map_err(|e| AppError::Redis(e))
    }

    pub async fn remove_user_from_chat_room(&self, session_id: &str, user_id: &str) -> Result<(), AppError> {
        let mut conn = self.manager.clone();
        conn.srem(format!("chat_room:{}", session_id), user_id)
            .await
            .map_err(|e| AppError::Redis(e))
    }

    pub async fn get_chat_room_users(&self, session_id: &str) -> Result<Vec<String>, AppError> {
        let mut conn = self.manager.clone();
        conn.smembers(format!("chat_room:{}", session_id))
            .await
            .map_err(|e| AppError::Redis(e))
    }

    // Whiteboard state management
    pub async fn set_whiteboard_state(&self, session_id: &str, state: &str) -> Result<(), AppError> {
        let mut conn = self.manager.clone();
        conn.hset(format!("whiteboard:{}", session_id), "state", state)
            .await
            .map_err(|e| AppError::Redis(e))
    }

    pub async fn get_whiteboard_state(&self, session_id: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.manager.clone();
        conn.hget(format!("whiteboard:{}", session_id), "state")
            .await
            .map_err(|e| AppError::Redis(e))
    }

    // Health check
    pub async fn health_check(&self) -> Result<(), AppError> {
        let mut conn = self.manager.clone();
        let _: String = conn.ping().await.map_err(|e| AppError::Redis(e))?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPresence {
    pub status: String,
    pub current_role: String,
    pub last_seen: i64,
}

// Redis key builders
pub struct RedisKeys;

impl RedisKeys {
    pub fn session(user_id: &str) -> String {
        format!("session:{}", user_id)
    }

    pub fn active_role(user_id: &str) -> String {
        format!("active_role:{}", user_id)
    }

    pub fn user_presence(user_id: &str) -> String {
        format!("user_presence:{}", user_id)
    }

    pub fn rate_limit(user_id: &str, endpoint: &str) -> String {
        format!("rate_limit:{}:{}", user_id, endpoint)
    }

    pub fn rate_limit_role(user_id: &str, role: &str, endpoint: &str) -> String {
        format!("rate_limit:{}:{}:{}", user_id, role, endpoint)
    }

    pub fn chat_room(session_id: &str) -> String {
        format!("chat_room:{}", session_id)
    }

    pub fn whiteboard(session_id: &str) -> String {
        format!("whiteboard:{}", session_id)
    }

    pub fn webrtc_signal(session_id: &str) -> String {
        format!("webrtc_signal:{}", session_id)
    }

    pub fn mentor_profile_cache(user_id: &str) -> String {
        format!("mentor_profile_cache:{}", user_id)
    }

    pub fn mentee_profile_cache(user_id: &str) -> String {
        format!("mentee_profile_cache:{}", user_id)
    }
}